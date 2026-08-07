import { ref } from "vue"

const showInstanceSettings = ref(false)

export function useInstanceSettings() {
    return showInstanceSettings
}
