import { Config } from "@conic/config"

declare global {
    interface Window {
        __APPLICATION_CONFIG__: Config
    }
}
