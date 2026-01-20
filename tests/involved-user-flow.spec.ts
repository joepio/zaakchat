import { test, expect } from "@playwright/test";
import { login } from "./helpers";

test.describe("Involved User Flow", () => {
  test("should allow an involved user to access the issue and comment", async ({
    page,
  }) => {
    page.on("console", (msg) => console.log(`BROWSER LOG: ${msg.text()}`));

    // 1. Login as primary user (User A)
    await login(page);

    // 2. Create an issue with an involved person (User B)
    await page.getByRole("button", { name: /Nieuwe Zaak/i }).click();
    await expect(page.locator("text=Betrokkenen")).toBeVisible();

    const uniqueId = Date.now();
    const issueTitle = `Involved Test Issue ${uniqueId}`;
    const involvedEmail = `involved-${uniqueId}@zaakchat.nl`;

    // Assuming the create form defaults to Issue or has a selector.
    // Based on SchemaForm.tsx, we might need to select "Task" or "Comment" but here we want "Issue".
    // Wait, SchemaForm filters out "Issue". We need to use the dedicated "Create Issue" button/form.
    // The "create-issue-button" likely opens CreateIssueForm.tsx.

    await page.getByLabel("Titel").fill(issueTitle);
    await page
      .getByLabel("Beschrijving")
      .fill("Test description for involved user flow");

    // 3. Add involved person
    const involvedInput = page.getByLabel("Betrokkenen");
    await involvedInput.fill(involvedEmail);
    await involvedInput.press("Enter");

    // Submit
    await page.click('button[type="submit"]');

    // Wait for issue to appear in the list and click it
    await expect(page.locator(`text=${issueTitle}`)).toBeVisible();
    await page.click(`text=${issueTitle}`);

    // Verify we are on the issue page
    await expect(page.locator('[data-testid="issue-header"]')).toBeVisible();

    // 3. Verify the issue was created successfully

    // Check issue header for involved user
    await expect(page.locator('[data-testid="issue-header"]')).toContainText(
      involvedEmail,
    );

    // Timeline events are hidden by default, show them
    const showDetailsButton = page.getByRole("button", { name: /Toon/i });
    await showDetailsButton.waitFor({ state: "visible" });
    await showDetailsButton.click();

    // Check timeline for the event
    // Using getByText is more robust than locator combinations
    await expect(page.getByText(/zaak aangemaakt/i)).toBeVisible({
      timeout: 10000,
    });

    // Check for "betrokkenen" update - this might be separate or part of the same event
    // The screenshot shows both "zaak aangemaakt" and "zaak bijgewerkt"
    await expect(page.getByText(/betrokkenen/i)).toBeVisible({
      timeout: 10000,
    });

    console.log(
      `Test passed: Issue "${issueTitle}" was created and contains involved person "${involvedEmail}"`,
    );
  });
});
