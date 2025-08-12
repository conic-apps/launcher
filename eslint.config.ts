import { globalIgnores } from "eslint/config"
import { defineConfigWithVueTs, vueTsConfigs } from "@vue/eslint-config-typescript"
import pluginVue from "eslint-plugin-vue"
import pluginVitest from "@vitest/eslint-plugin"
import skipFormatting from "@vue/eslint-config-prettier/skip-formatting"

import { TSESLint } from "@typescript-eslint/utils"

const config: TSESLint.FlatConfig.Config[] = defineConfigWithVueTs(
    {
        name: "app/files-to-lint",
        files: ["**/*.{ts,mts,tsx,vue}"],
    },

    globalIgnores(["**/dist/**", "**/dist-ssr/**", "**/coverage/**"]),

    pluginVue.configs["flat/essential"],
    vueTsConfigs.recommended,

    {
        ...pluginVitest.configs.recommended,
        files: ["src/**/__tests__/*"],
    },
    skipFormatting,
    {
        files: ["src/**"],
        rules: {
            "vue/multi-word-component-names": "off",
        },
    },
    {
        files: ["src/**"],
        rules: {
            "@typescript-eslint/naming-convention": [
                "warn",
                {
                    selector: "variable",
                    modifiers: ["const"],
                    format: ["camelCase"],
                },
                {
                    selector: "variable",
                    modifiers: ["const"],
                    filter: {
                        regex: "^[A-Z0-9_]+$",
                        match: true,
                    },
                    format: ["UPPER_CASE"],
                },
                {
                    selector: "variableLike",
                    format: ["camelCase"],
                },
                {
                    selector: "typeLike",
                    format: ["PascalCase"],
                },
                {
                    selector: "interface",
                    format: ["PascalCase"],
                },
            ],
        },
    },
)

export default config
