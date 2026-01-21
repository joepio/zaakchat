import { test, expect } from "@playwright/test";
import { login } from "./helpers";

test.describe("Theme and Color Mode Switcher", () => {
  test("should switch theme, color mode and persist preferences", async ({
    page,
  }) => {
    await login(page);

    // 1. Check default theme (Rijksoverheid)
    // The logo-primary should be the default Primary color
    const h1 = page.locator('[data-testid="main-heading"]');
    // Rijksoverheid Primary in code is #007bc7 -> rgb(0, 123, 199)
    await expect(h1).toHaveCSS("color", "rgb(0, 123, 199)");

    // 2. Open settings
    await page.getByTitle("Instellingen").click();
    await expect(page.getByText("User Settings")).toBeVisible();

    // 3. Switch to Utrecht
    await page.locator("#theme").selectOption("utrecht");

    // 4. Verify theme change (Utrecht Primary is #cc0100 -> rgb(204, 1, 0))
    await expect(h1).toHaveCSS("color", "rgb(204, 1, 0)");

    // 5. Verify sharp corners (border-radius: 0)
    const createButton = page.getByTestId("create-issue-button");
    await expect(createButton).toHaveCSS("border-radius", "0px");

    // 6. Switch color mode to Dark
    await page.locator("#color-mode").selectOption("dark");

    // 7. Verify dark mode applied (bg-primary becomes #0a0a0a -> rgb(10, 10, 10))
    const body = page.locator("body");
    await expect(body).toHaveCSS("background-color", "rgb(10, 10, 10)");

    // 8. Refresh and verify persistence
    await page.reload();
    await expect(h1).toHaveCSS("color", "rgb(204, 1, 0)");
    await expect(createButton).toHaveCSS("border-radius", "0px");
    await expect(body).toHaveCSS("background-color", "rgb(10, 10, 10)");
  });
});
