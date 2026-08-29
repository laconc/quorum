import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Phase 0 has no product in it. These tests prove the harness works end to end,
 * so that the phases which do have a product inherit a pipeline that has
 * already been shown to catch things.
 */

/** Every route the application serves. */
const ROUTES = ["/"];

test.describe("the harness", () => {
  test("renders the page", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText("The harness works");
  });

  test("renders the frozen clock", async ({ page }) => {
    await page.goto("/");
    // If this ever shows a moving time, the screenshot pipeline is no longer
    // deterministic and every image will churn.
    await expect(page.getByText("2026-03-01T12:00:00Z")).toBeVisible();
  });

  test("swaps a fragment into a live region", async ({ page }) => {
    await page.goto("/");

    const result = page.locator("#result");
    // The live region exists from first paint. One created at the same moment
    // as its content is not announced at all, which is the mistake this
    // asserts against.
    await expect(result).toHaveAttribute("aria-live", "polite");
    await expect(result).toBeEmpty();

    await page.getByRole("button", { name: "Run the check" }).click();
    await expect(result).toContainText("Checked.");
  });

  test("serves assets from fingerprinted, immutable URLs", async ({ page }) => {
    const responses: { url: string; cacheControl: string | null }[] = [];
    page.on("response", (r) => {
      if (r.url().includes("/static/")) {
        responses.push({ url: r.url(), cacheControl: r.headers()["cache-control"] ?? null });
      }
    });

    await page.goto("/");
    expect(responses.length).toBeGreaterThan(0);
    for (const { url, cacheControl } of responses) {
      expect(url).toMatch(/\/static\/[0-9a-f]{16}\//);
      expect(cacheControl).toContain("immutable");
    }
  });

  test("loads no third-party origin", async ({ page }) => {
    // The vendored script is the point: a content delivery network would need
    // admitting to the Content Security Policy, and would fail the application
    // in exactly the conditions where it matters most.
    const external: string[] = [];
    page.on("request", (r) => {
      const url = new URL(r.url());
      if (url.hostname !== "127.0.0.1" && url.hostname !== "localhost") {
        external.push(r.url());
      }
    });

    await page.goto("/");
    await page.getByRole("button", { name: "Run the check" }).click();
    await expect(page.locator("#result")).toContainText("Checked.");
    expect(external).toEqual([]);
  });

  test("reports no content security policy violations", async ({ page }) => {
    const violations: string[] = [];
    page.on("console", (msg) => {
      if (msg.text().includes("Content Security Policy")) {
        violations.push(msg.text());
      }
    });

    await page.goto("/");
    await page.getByRole("button", { name: "Run the check" }).click();
    await expect(page.locator("#result")).toContainText("Checked.");
    expect(violations).toEqual([]);
  });
});

test.describe("accessibility @a11y", () => {
  for (const route of ROUTES) {
    test(`${route} has no axe violations`, async ({ page }) => {
      await page.goto(route);
      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
        .analyze();

      // Automated tooling catches roughly a third of real accessibility
      // problems. Clean here is a floor, not a verdict — the VoiceOver session
      // in Phase 2 is where the findings that matter come from.
      expect(results.violations).toEqual([]);
    });
  }

  test("the page body never scrolls horizontally", async ({ page }) => {
    await page.goto("/");
    const overflows = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
    );
    expect(overflows).toBe(false);
  });

  test("reflows at 320px without horizontal scroll", async ({ page }) => {
    // WCAG 1.4.10 Reflow, stated the way the criterion actually is: content at
    // 320 CSS pixels wide, which is what 1280px at 400% zoom comes to. Testing
    // it by viewport rather than by injecting a stylesheet is both closer to
    // the criterion and possible under a policy with no unsafe-inline — an
    // injected style is refused, which is the policy doing its job.
    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto("/");

    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByRole("button", { name: "Run the check" })).toBeVisible();

    const overflows = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
    );
    expect(overflows).toBe(false);
  });

  test("every interactive control is keyboard reachable with visible focus", async ({
    page,
  }, testInfo) => {
    // Emulated touch devices do not move focus on Tab the way a real keyboard
    // does, so this is a desktop assertion. The property still holds
    // everywhere — it is the emulation that cannot express it.
    test.skip(
      testInfo.project.name === "webkit-mobile",
      "Tab does not move focus under touch emulation",
    );

    await page.goto("/");
    await page.keyboard.press("Tab");
    const button = page.getByRole("button", { name: "Run the check" });
    await expect(button).toBeFocused();

    const outline = await button.evaluate((el) => getComputedStyle(el).outlineStyle);
    expect(outline).not.toBe("none");
  });
});

/**
 * One engine per viewport. Three projects run the functional and accessibility
 * checks, but two of them share a 375px width, and both writing
 * `harness-check-375.png` would mean whichever finished last decided the
 * gallery. Desktop images come from Chromium; the 375px images come from
 * WebKit, because iOS Safari is what this audience actually holds.
 *
 * The width is taken from this table rather than from the live viewport, so a
 * test that resized the page cannot rename someone else's file.
 */
const SCREENSHOT_PROJECTS: Record<string, number> = {
  "chromium-desktop": 1280,
  "webkit-mobile": 375,
};

/** The gallery lives at the repository root, not under e2e/. */
const GALLERY = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "docs",
  "screenshots",
);

test.describe("screenshots @screenshot", () => {
  test("harness page", async ({ page }, testInfo) => {
    const width = SCREENSHOT_PROJECTS[testInfo.project.name];
    test.skip(
      width === undefined,
      "one engine per viewport, so two projects cannot race for one filename",
    );

    await page.goto("/");
    // Wait for the script to have been applied, so the image is of a settled
    // page rather than a race.
    await page.waitForFunction(() => "htmx" in window);

    // <surface>-<screen>-<viewport>.png
    const name = `harness-check-${width}.png`;

    await page.screenshot({
      path: path.join(GALLERY, name),
      fullPage: true,
      animations: "disabled",
      caret: "hide",
    });
  });
});
