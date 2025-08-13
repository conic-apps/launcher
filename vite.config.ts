import { fileURLToPath, URL } from "node:url"

import { defineConfig } from "vite"
import vue from "@vitejs/plugin-vue"
import vueDevTools from "vite-plugin-vue-devtools"
import svgLoader from "vite-svg-loader"

export default defineConfig({
    plugins: [vue(), vueDevTools(), svgLoader()],
    resolve: {
        alias: {
            "@": fileURLToPath(new URL("./src", import.meta.url)),
            "@conic": fileURLToPath(new URL("./crates", import.meta.url)),
        },
    },
    server: {
        watch: {
            ignored: ["**/target/**"],
        },
    },
    build: {
        minify: "esbuild",
        target: "chrome89",
    },
})
