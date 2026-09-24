import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In development the game server runs on :3000 and is reached through this proxy.
// In production, VITE_SERVER_URL points at the deployed server.
export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    proxy: {
      "/api": { target: "http://localhost:3000", ws: true },
    },
  },
});
