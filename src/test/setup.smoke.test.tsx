import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

// Smoke test that verifies the Vitest + jsdom + React Testing Library
// + jest-dom toolchain is wired up correctly. Safe to delete once real
// component tests exist.
describe("vitest environment", () => {
  it("runs basic assertions", () => {
    expect(1 + 1).toBe(2);
  });

  it("renders React components into a jsdom DOM", () => {
    render(<button type="button">Click me</button>);
    expect(
      screen.getByRole("button", { name: "Click me" }),
    ).toBeInTheDocument();
  });
});
