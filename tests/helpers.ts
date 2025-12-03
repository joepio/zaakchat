import { Page, APIRequestContext, expect } from "@playwright/test";

export async function resetServerState(request: APIRequestContext) {
  const response = await request.post("/reset/");
  expect(response.ok()).toBeTruthy();
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
}
