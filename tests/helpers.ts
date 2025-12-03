import { Page, APIRequestContext, expect } from "@playwright/test";

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
      const issues = document.querySelectorAll('.zaak-item-hover');
      const noIssues = document.querySelector('[data-testid="no-issues"]');
      return issues.length > 0 || noIssues !== null;
    },
    { timeout }
  );
}

export async function waitForIssueDetailPage(page: Page, timeout = 10000) {
  // Wait for issue detail page to load via SSE
  // Check for the issue header which indicates the page has loaded
  await page.waitForSelector('[data-testid="issue-header"]', {
    state: 'visible',
    timeout
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

  await page.fill('input[type="email"]', "test-user@example.com");
  await page.click('button[type="submit"]');

  // Wait for dashboard to load
  await expect(page.locator('[data-testid="main-heading"]')).toBeVisible();

  // Wait for SSE connection to establish and load initial data
  await waitForSSEConnection(page);
}
