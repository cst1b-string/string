import react from "@vitejs/plugin-react-swc";
import path from "path";
import { defineConfig } from "vite";
import eslint from "vite-plugin-eslint";

export default defineConfig({
	// prevent vite from obscuring rust errors
	clearScreen: false,
	// Tauri expects a fixed port, fail if that port is not available
	server: {
		strictPort: true,
	},
	envPrefix: ["VITE_", "TAURI_"],
	build: {
		// Tauri uses Chromium on Windows and WebKit on macOS and Linux
		target: process.env.TAURI_PLATFORM === "windows" ? "chrome105" : "safari13",
		// don't minify for debug builds
		minify: process.env.TAURI_DEBUG === undefined ? "esbuild" : false,
		// produce sourcemaps for debug builds
		sourcemap: !(process.env.TAURI_DEBUG === undefined),
	},
	plugins: [react()],
	resolve: {
		alias: {
			// eslint-disable-next-line @typescript-eslint/naming-convention
			"@": path.resolve(__dirname, "./src"),
		},
	},
});
