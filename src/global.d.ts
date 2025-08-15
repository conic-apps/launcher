import { Config } from "@conic/config"

declare global {
    interface Window {
        __CONIC_CONFIG__: Config
    }
}
