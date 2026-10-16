import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/target/**"],
    },
  },
});
