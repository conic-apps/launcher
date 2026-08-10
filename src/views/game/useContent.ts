import { ref } from "vue"
import { ComponentName } from "./Content.vue"

const componentName = ref<ComponentName>("saves")

const showContent = ref({
    saves: false,
    mods: false,
    resourcepacks: false,
    screenshots: false,
})

export function useShowContent() {
    return showContent
}
