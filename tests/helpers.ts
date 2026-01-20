import { Page, APIRequestContext, expect } from "@playwright/test";
import * as fs from "fs";
import * as path from "path";

export async function resetServerState(request: APIRequestContext) {
  const response = await request.post("/reset/");
  expect(response.ok()).toBeTruthy();
}

export async function waitForSSEConnection(page: Page, timeout = 10000) {
  // Wait for SSE connection to be established by checking if issues are loaded
  // This ensures the connection is ready before tests proceed
  await page.waitForFunction(
    () => {
      // Check if we have any issues loaded OR the empty state message
      const issues = document.querySelectorAll(".zaak-item-hover");
      const noIssues = document.querySelector('[data-testid="no-issues"]');
      return issues.length > 0 || noIssues !== null;
    },
    { timeout },
  );
}

export async function waitForIssueDetailPage(page: Page, timeout = 10000) {
  // Wait for issue detail page to load via SSE
  // Check for the issue header which indicates the page has loaded
  await page.waitForSelector('[data-testid="issue-header"]', {
    state: "visible",
    timeout,
  });
}

export async function login(page: Page) {
  // Reset server state first to ensure clean slate
  await resetServerState(page.request);

  await page.goto("/");

  // Check if already logged in
  const isLoginVisible = await page.isVisible('input[type="email"]');
  if (!isLoginVisible) {
    return;
  }

  await page.fill('input[type="email"]', "test-user@zaakchat.nl");
  await page.click('button[type="submit"]');

  // In test mode, poll for the mock email file and navigate to the magic link
  const mockPath = path.join(process.cwd(), "test_email.json");
  let magicLink = "";

  // Poll for up to 5 seconds (10 * 500ms)
  for (let i = 0; i < 10; i++) {
    try {
      if (fs.existsSync(mockPath)) {
        const data = await fs.promises.readFile(mockPath, "utf8");
        const json = JSON.parse(data);
        if (json.magic_link) {
          magicLink = json.magic_link;
          // Delete the file to clean up and avoid stale data
          await fs.promises.unlink(mockPath);
          break;
        }
      }
    } catch (e) {
      // Ignore errors during polling
    }
    await page.waitForTimeout(500);
  }

  if (magicLink) {
    // Fix: Ensure magic link uses the same origin as the current page (e.g. handle 5173 vs 5174)
    if (!process.env.CI) {
      const url = new URL(magicLink);
      const currentOrigin = new URL(page.url()).origin;
      url.protocol = new URL(currentOrigin).protocol;
      url.host = new URL(currentOrigin).host;
      magicLink = url.toString();
    }
    await page.goto(magicLink);
    // Wait for dashboard to load AFTER verification
    await expect(page.locator('[data-testid="main-heading"]')).toBeVisible();
    // Wait for SSE connection to establish and load initial data
    await waitForSSEConnection(page);
  } else {
    console.warn(
      "[helpers] No mock email file found after polling, proceeding without auto-login.",
    );
    // If we didn't auto-login, we might still be on the login page or dashboard if already logged in (checked at start)
    // But if we are here, it means we tried to login and failed to find the email.
    // We can try to wait for dashboard anyway in case we were already logged in (but the check at start should have caught that)
  }
}

export async function getApiAuthToken(
  request: APIRequestContext,
  email = "test-user@zaakchat.nl",
) {
  // 1. Initiate login
  const loginRes = await request.post("/login", {
    data: { email },
  });
  expect(loginRes.ok()).toBeTruthy();

  // 2. Poll for mock email
  const mockPath = path.join(process.cwd(), "test_email.json");
  let token = "";

  for (let i = 0; i < 10; i++) {
    try {
      if (fs.existsSync(mockPath)) {
        const data = await fs.promises.readFile(mockPath, "utf8");
        const json = JSON.parse(data);
        if (json.token) {
          token = json.token;
          // Delete the file
          await fs.promises.unlink(mockPath);
          break;
        }
      }
    } catch (e) {
      // Ignore
    }
    // Wait 500ms
    await new Promise((r) => setTimeout(r, 500));
  }

  if (!token) {
    throw new Error("Failed to get token from mock email");
  }

  // 3. Verify token to get JWT
  const verifyRes = await request.get(`/auth/verify?token=${token}`);
  expect(verifyRes.ok()).toBeTruthy();
  const data = await verifyRes.json();
  return data.token;
}
