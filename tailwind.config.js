/** @type {import('tailwindcss').Config} */
const ink = (n) => `rgb(var(--ink-${n}) / <alpha-value>)`;

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: ['"Segoe UI Variable Text"', '"Segoe UI"', "system-ui", "sans-serif"],
        display: ['"Segoe UI Variable Display"', '"Segoe UI"', "system-ui", "sans-serif"],
        mono: ['"Cascadia Mono"', "Consolas", "ui-monospace", "monospace"],
      },
      colors: {
        ink: Object.fromEntries(
          [50, 100, 200, 300, 400, 500, 600, 700, 800, 850, 900, 950].map((n) => [n, ink(n)]),
        ),
        // Every "accent" utility follows the colour chosen in the settings.
        accent: "rgb(var(--accent) / <alpha-value>)",
      },
      animation: {
        "fade-in": "fadeIn 0.15s ease-out",
        "scale-in": "scaleIn 0.15s cubic-bezier(0.16, 1, 0.3, 1)",
      },
      keyframes: {
        fadeIn: { "0%": { opacity: "0" }, "100%": { opacity: "1" } },
        scaleIn: {
          "0%": { opacity: "0", transform: "scale(0.98)" },
          "100%": { opacity: "1", transform: "scale(1)" },
        },
      },
    },
  },
  plugins: [],
};
