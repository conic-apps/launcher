import { computed, nextTick, ref, watch } from "vue"
import { useInstanceStore } from "@/store/instance"
import { getMinecrafVersionManifest } from "@conic/install"

const PAGE_SIZE = 20
const VERSIONS_PER_PAGE = 6

export function useSearchPagination(
    totalPages: () => number,
    selectedVersions: { value: string[] },
) {
    const instanceStore = useInstanceStore()

    const searchQuery = ref("")
    const currentPage = ref(1)
    const versionOptions = ref<string[]>([])
    const versionPage = ref(0)
    const versionOffset = ref(0)
    const versionPageAnimated = ref(false)
    let versionTrackElement: HTMLElement | null = null

    function setVersionTrackRef(el: unknown) {
        versionTrackElement = el instanceof HTMLElement ? el : null
    }

    function toggleFilterOption<T>(list: T[], value: T) {
        const index = list.indexOf(value)
        if (index >= 0) {
            list.splice(index, 1)
        } else {
            list.push(value)
        }
    }

    const paginationPages = computed(() => {
        const total = totalPages()
        const current = currentPage.value
        const pages: (number | "\u2026")[] = []
        if (total <= 15) {
            for (let page = 1; page <= total; page++) pages.push(page)
            return pages
        }
        if (current <= 7) {
            for (let page = 1; page <= 13; page++) pages.push(page)
            pages.push("\u2026")
            pages.push(total)
            return pages
        }
        if (current >= total - 6) {
            pages.push(1)
            pages.push("\u2026")
            for (let page = total - 12; page <= total; page++) pages.push(page)
            return pages
        }
        pages.push(1)
        pages.push("\u2026")
        for (let page = current - 5; page <= current + 5; page++) pages.push(page)
        pages.push("\u2026")
        pages.push(total)
        return pages
    })

    const versionPageCount = computed(() =>
        Math.max(0, Math.ceil(versionOptions.value.length / VERSIONS_PER_PAGE)),
    )

    const versionTrackStyle = computed(() => ({
        transform: `translateX(${versionOffset.value}px)`,
        transition: versionPageAnimated.value ? "transform 240ms ease" : "none",
    }))

    async function updateVersionOffset() {
        await nextTick()
        const track = versionTrackElement
        if (!track) return
        const chips = track.querySelectorAll(".filter-chip")
        if (chips.length === 0) {
            versionOffset.value = 0
            return
        }
        const index = Math.min(versionPage.value * VERSIONS_PER_PAGE, chips.length - 1)
        const chip = chips[index] as HTMLElement
        versionOffset.value = -chip.offsetLeft
        if (!versionPageAnimated.value) {
            requestAnimationFrame(() => {
                versionPageAnimated.value = true
            })
        }
    }

    function versionPagePrev() {
        versionPage.value = Math.max(0, versionPage.value - 1)
    }

    function versionPageNext() {
        versionPage.value = Math.min(versionPageCount.value - 1, versionPage.value + 1)
    }

    function syncVersionPageToSelection() {
        const current = selectedVersions.value[0]
        if (!current) {
            versionPage.value = 0
            return
        }
        const index = versionOptions.value.indexOf(current)
        versionPage.value = index >= 0 ? Math.floor(index / VERSIONS_PER_PAGE) : 0
    }

    function searchInitKey(): string {
        const runtime = instanceStore.currentInstance?.config.runtime
        return `${runtime?.mod_loader_type ?? ""}|${runtime?.minecraft ?? ""}`
    }

    async function loadVersionOptions() {
        try {
            const manifest = await getMinecrafVersionManifest()
            const options = manifest.versions
                .filter((version) => version.type === "release")
                .sort(
                    (a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime(),
                )
                .map((version) => version.id)
            const minecraft = instanceStore.currentInstance?.config.runtime.minecraft
            if (minecraft && !options.includes(minecraft)) {
                options.push(minecraft)
            }
            versionOptions.value = options
        } catch (error) {
            console.error(error)
        }
    }

    watch(versionPage, () => {
        void updateVersionOffset()
    })

    return {
        PAGE_SIZE,
        searchQuery,
        currentPage,
        versionOptions,
        versionPage,
        versionOffset,
        setVersionTrackRef,
        toggleFilterOption,
        paginationPages,
        versionPageCount,
        versionTrackStyle,
        updateVersionOffset,
        versionPagePrev,
        versionPageNext,
        syncVersionPageToSelection,
        searchInitKey,
        loadVersionOptions,
        instanceStore,
    }
}
