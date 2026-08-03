"use strict";

/**
 * Sets a React-controlled <input>/<textarea>'s value the way React actually
 * observes it: through the native value setter (bypassing React's own
 * setter override) followed by a real 'input' event, so the component's
 * onChange fires exactly once with the full final value — instead of relying
 * on WebDriver's low-level clear()/sendKeys(), which can desync from a
 * controlled component that re-renders (and can rewrite the DOM value) on
 * every keystroke.
 */
async function setReactValue(driver, element, value) {
  await driver.executeScript(
    (el, val) => {
      const proto = el.tagName === "TEXTAREA" ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(proto, "value").set;
      setter.call(el, val);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    },
    element,
    value,
  );
}

/**
 * Clicks an element via the DOM directly instead of a real WebDriver mouse
 * click. Two situations where this matters here: hover-close dropdown menus
 * (a native click moves the cursor across the menu first, which can fire the
 * mouseleave that closes it before the click itself lands), and elements
 * transiently covered by an unrelated toast notification (native clicks
 * refuse to click something else is on top, even though the app doesn't
 * actually care what's visually overlapping for a programmatic .click()).
 */
async function jsClick(driver, element) {
  await driver.executeScript((el) => el.click(), element);
}

module.exports = { setReactValue, jsClick };
