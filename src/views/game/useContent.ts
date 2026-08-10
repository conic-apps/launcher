import { ref } from "vue"

const showContent = ref({
    saves: false,
    mods: false,
    resourcepacks: false,
    screenshots: false,
})

export function useShowContent() {
    return showContent
}
