declare const __APP_VERSION__: string;

/** Style sheets are bundled by Vite. */
declare module "*.css";
declare module "*?raw" {
  const text: string;
  export default text;
}
