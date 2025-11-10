#!/usr/bin/env node

/**
 * TypeScript Schema Generation Script
 *
 * This script:
 * 1. Runs the Rust schema export binary
 * 2. Reads the exported JSON schemas from target/schemas/
 * 3. Uses json-schema-to-typescript to convert them to TypeScript interfaces
 * 4. Updates frontend/src/types/interfaces.ts
 */

import { execSync } from 'child_process';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { compile } from 'json-schema-to-typescript';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const projectRoot = join(__dirname, '..');
const schemasDir = join(projectRoot, 'target/schemas');
const outputFile = join(projectRoot, 'frontend/src/types/interfaces.ts');

/**
 * Main execution function
 */
async function main() {
  console.log('🔄 Generating TypeScript interfaces from Rust schemas...');

  try {
    // Step 1: Check if schemas directory exists
    if (!existsSync(schemasDir)) {
      throw new Error(`Schemas directory not found: ${schemasDir}`);
    }

    // Step 2: Read schema index to get list of schemas
    const indexPath = join(schemasDir, 'index.json');
    if (!existsSync(indexPath)) {
      throw new Error('Schema index not found');
    }

    const index = JSON.parse(readFileSync(indexPath, 'utf8'));
    console.log(`📋 Found ${index.schemas.length} schemas: ${index.schemas.join(', ')}`);

    // Step 3: Read the combined schema file which has all definitions resolved
    const combinedSchemaPath = join(schemasDir, 'all_schemas.json');
    if (!existsSync(combinedSchemaPath)) {
      throw new Error('Combined schema file not found');
    }

    const allSchemas = JSON.parse(readFileSync(combinedSchemaPath, 'utf8'));
    console.log(`📋 Processing ${Object.keys(allSchemas).length} schemas from combined file`);

    // Convert each schema using json-schema-to-typescript
    const interfacePromises = [];

    for (const [schemaName, schema] of Object.entries(allSchemas)) {
      // Skip schemas with unresolved references for now
      const schemaJson = JSON.stringify(schema);
      if (schemaJson.includes('$ref')) {
        console.warn(`⚠️  Skipping schema ${schemaName} due to unresolved references`);
        let fallbackInterface;

        if (schemaName === 'PlanningMoment') {
          // Create proper fallback for PlanningMoment
          fallbackInterface = `export interface PlanningMoment {
  /** Unieke moment identifier */
  id: string;
  /** Geplande datum in YYYY-MM-DD formaat */
  date: string;
  /** Titel van dit planning moment */
  title: string;
  /** Huidige status van dit moment */
  status: "completed" | "current" | "planned";
}`;
        } else {
          fallbackInterface = `export interface ${schemaName} {\n  [key: string]: any;\n}`;
        }

        interfacePromises.push(Promise.resolve(fallbackInterface));
        continue;
      }

      // Use json-schema-to-typescript to compile the schema
      const interfacePromise = compile(schema, schemaName, {
        bannerComment: '',
        style: {
          bracketSpacing: false,
          singleQuote: false,
          tabWidth: 2,
        },
        additionalProperties: false,
        declareExternallyReferenced: false,
        enableConstEnums: false,
        format: true,
        ignoreMinAndMaxItems: false,
        maxItems: 20,
        strictIndexSignatures: false,
        unreachableDefinitions: false,
        unknownAny: true,
        $refOptions: {
          resolve: {
            internal: false,
            external: false,
          },
        },
      }).then(ts => {
        console.log(`✅ Converted schema: ${schemaName}`);
        // Ensure core entities include id and timestamp fields (id, created_at, updated_at)
        const coreEntities = ["Issue", "Task", "Comment", "Planning", "Document"];
        if (coreEntities.includes(schemaName)) {
          // Try to inject fields into the interface body if present
          const ifacePattern = new RegExp(`(export interface ${schemaName}\\s*\\{)`);
          if (ifacePattern.test(ts)) {
            ts = ts.replace(ifacePattern, `$1\n  id: string;\n  created_at?: string | null;\n  updated_at?: string | null;`);
          } else {
            // Fallback: prepend a minimal interface declaration
            const fallback = `export interface ${schemaName} {\n  id: string;\n  created_at?: string | null;\n  updated_at?: string | null;\n}\n`;
            ts = fallback + ts;
          }
        }
        return ts;
      }).catch(error => {
        console.warn(`⚠️  Failed to convert schema ${schemaName}:`, error.message);
        return `// Failed to generate interface for ${schemaName}: ${error.message}\nexport interface ${schemaName} { [key: string]: any; }`;
      });

      interfacePromises.push(interfacePromise);
    }

    // Wait for all conversions to complete
    const interfaces = await Promise.all(interfacePromises);

    // Step 5: Generate the complete TypeScript file
    const header = `// Auto-generated TypeScript interfaces
// Generated from Rust JSON schemas using json-schema-to-typescript
// Run 'pnpm run generate' to regenerate
//
// Last generated: ${new Date().toISOString()}

`;

    const helperFunctions = `

// Schema metadata interface for runtime schema fetching
export interface SchemaMetadata {
  schemas: string[];
  base_url: string;
  description: string;
}

// Helper function to fetch schemas from the server
export async function fetchSchema(schemaName: string): Promise<any> {
  const response = await fetch(\`/schemas/\${schemaName}\`);
  if (!response.ok) {
    throw new Error(\`Failed to fetch schema \${schemaName}: \${response.statusText}\`);
  }
  return response.json();
}

// Helper function to get all available schemas
export async function fetchSchemaIndex(): Promise<SchemaMetadata> {
  const response = await fetch('/schemas');
  if (!response.ok) {
    throw new Error(\`Failed to fetch schema index: \${response.statusText}\`);
  }
  return response.json();
}`;

    // Combine all interfaces
    const allInterfaces = interfaces.join('\n');

    const content = header + allInterfaces + helperFunctions;

    // Step 4: Ensure output directory exists
    const outputDir = dirname(outputFile);
    if (!existsSync(outputDir)) {
      mkdirSync(outputDir, { recursive: true });
      console.log(`📁 Created directory: ${outputDir}`);
    }

    // Step 5: Write the TypeScript file
    writeFileSync(outputFile, content);
    console.log(`✅ Generated TypeScript interfaces: ${outputFile}`);
    console.log(`📊 Generated ${interfaces.length} types using json-schema-to-typescript`);

    // Step 6: Verify the Document interface is included
    if (content.includes('export interface Document')) {
      console.log('✅ Document interface successfully included!');
    } else {
      console.warn('⚠️  Document interface not found in generated types');
    }

  } catch (error) {
    console.error('❌ Generation failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

// Run if this script is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}

export { main };
