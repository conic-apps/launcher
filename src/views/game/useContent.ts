import { ref } from "vue"
import { ComponentName } from "./Content.vue"

const componentName = ref<ComponentName>("saves")

export function useContentComponent() {
    return componentName
}

const showContent = ref(false)

export function useShowContent() {
    return showContent
}
