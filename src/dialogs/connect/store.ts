import { ref } from "vue"

export const currentComponent = ref<"downloadDescription" | "downloadProgress" | "connectManager">(
    "downloadDescription",
)
