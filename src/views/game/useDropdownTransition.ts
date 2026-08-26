// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { onMounted, onUnmounted, watch, type Ref } from "vue"

const CHEVRON_FLIP_DURATION = 200

export type DropdownTransitionOptions = {
    expandDuration?: number
    collapseDuration?: number
    expandEasing?: string
    collapseEasing?: string
    onChange?: (opened: boolean) => void
}

/**
 * Height/opacity expand & collapse animation shared by every dropdown menu.
 *
 * The animation can be interrupted at any point: re-opening while the menu is
 * still collapsing continues from the current height instead of jumping back
 * to fully closed.
 */
export function useDropdownTransition(
    opened: Ref<boolean>,
    options: DropdownTransitionOptions = {},
) {
    const settings = {
        expandDuration: options.expandDuration ?? 200,
        collapseDuration: options.collapseDuration ?? 200,
        expandEasing: options.expandEasing ?? "ease",
        collapseEasing: options.collapseEasing ?? "ease",
    }

    let pendingHeight: number | null = null
    let pendingOpacity = "1"
    let leavingEl: HTMLElement | null = null
    let lastCollapseHeight: number | null = null
    let lastCollapseOpacity = "1"
    let collapseRaf = 0

    function captureInterruptedCollapse() {
        cancelAnimationFrame(collapseRaf)
        collapseRaf = 0
        leavingEl = null
        if (lastCollapseHeight !== null) {
            if (pendingHeight === null) {
                pendingHeight = Math.round(lastCollapseHeight)
                pendingOpacity = lastCollapseOpacity
            }
            lastCollapseHeight = null
        }
    }

    function resetTransitionStyles(el: HTMLElement) {
        el.style.height = ""
        el.style.opacity = ""
        el.style.overflow = ""
    }

    function onBeforeEnter(el: Element) {
        const target = el as HTMLElement
        if (pendingHeight !== null) {
            target.style.height = `${pendingHeight}px`
            target.style.overflow = "hidden"
        } else {
            target.style.opacity = "0"
        }
    }

    function onEnter(el: Element, done: () => void) {
        const target = el as HTMLElement
        if (pendingHeight !== null) {
            const startHeight = pendingHeight
            const startOpacity = pendingOpacity
            pendingHeight = null
            pendingOpacity = "1"
            target.style.height = ""
            target.style.overflow = ""
            const targetHeight = target.offsetHeight
            target.style.height = `${startHeight}px`
            target.style.overflow = "hidden"
            void target.offsetHeight
            const animation = target.animate(
                [
                    { height: `${startHeight}px`, opacity: startOpacity },
                    { height: `${targetHeight}px`, opacity: "1" },
                ],
                { duration: settings.expandDuration, easing: settings.expandEasing },
            )
            animation.onfinish = () => {
                resetTransitionStyles(target)
                done()
            }
            return
        }
        const targetHeight = target.offsetHeight
        target.style.height = "0px"
        target.style.overflow = "hidden"
        void target.offsetHeight
        const animation = target.animate(
            [
                { height: "0px", opacity: "0" },
                { height: `${targetHeight}px`, opacity: "1" },
            ],
            { duration: settings.expandDuration, easing: settings.expandEasing },
        )
        animation.onfinish = () => {
            resetTransitionStyles(target)
            done()
        }
    }

    function onAfterEnter(el: Element) {
        resetTransitionStyles(el as HTMLElement)
    }

    function onEnterCancelled(el: Element) {
        const target = el as HTMLElement
        const currentHeight = target.offsetHeight
        const currentOpacity = getComputedStyle(target).opacity
        target.getAnimations().forEach((animation) => animation.cancel())
        target.style.height = `${currentHeight}px`
        target.style.opacity = currentOpacity
        target.style.overflow = "hidden"
    }

    function onBeforeLeave(el: Element) {
        const target = el as HTMLElement
        target.style.height = `${target.offsetHeight}px`
        target.style.overflow = "hidden"
    }

    function onLeave(el: Element, done: () => void) {
        const target = el as HTMLElement
        target.getAnimations().forEach((animation) => animation.cancel())
        const startHeight = target.offsetHeight
        const startOpacity = getComputedStyle(target).opacity
        leavingEl = target
        lastCollapseHeight = startHeight
        lastCollapseOpacity = startOpacity
        const animation = target.animate(
            [
                { height: `${startHeight}px`, opacity: startOpacity },
                { height: "0px", opacity: "0" },
            ],
            { duration: settings.collapseDuration, easing: settings.collapseEasing },
        )
        const track = () => {
            lastCollapseHeight = target.offsetHeight
            lastCollapseOpacity = getComputedStyle(target).opacity
            collapseRaf = requestAnimationFrame(track)
        }
        collapseRaf = requestAnimationFrame(track)
        animation.onfinish = () => {
            cancelAnimationFrame(collapseRaf)
            collapseRaf = 0
            leavingEl = null
            lastCollapseHeight = null
            target.style.height = "0px"
            target.style.opacity = "0"
            done()
        }
    }

    function onAfterLeave() {
        captureInterruptedCollapse()
    }

    function onLeaveCancelled(el: Element) {
        const target = el as HTMLElement
        target.getAnimations().forEach((animation) => animation.cancel())
        captureInterruptedCollapse()
        resetTransitionStyles(target)
    }

    watch(opened, (value) => {
        options.onChange?.(value)
        if (value && leavingEl && leavingEl.isConnected) {
            pendingHeight = Math.round(leavingEl.offsetHeight)
            pendingOpacity = getComputedStyle(leavingEl).opacity
            leavingEl = null
        } else if (!value) {
            pendingHeight = null
            pendingOpacity = "1"
        }
    })

    onUnmounted(() => {
        cancelAnimationFrame(collapseRaf)
        collapseRaf = 0
    })

    return {
        onBeforeEnter,
        onEnter,
        onAfterEnter,
        onEnterCancelled,
        onBeforeLeave,
        onLeave,
        onAfterLeave,
        onLeaveCancelled,
    }
}

/** Closes the dropdown whenever the pointer goes down outside of `element`. */
export function useDismissOnOutsidePointerDown(
    element: Ref<HTMLElement | null>,
    opened: Ref<boolean>,
) {
    function onPointerDownOutside(event: PointerEvent) {
        const target = event.target as HTMLElement
        if (element.value && !element.value.contains(target)) {
            opened.value = false
        }
    }

    onMounted(() => {
        document.addEventListener("pointerdown", onPointerDownOutside)
    })

    onUnmounted(() => {
        document.removeEventListener("pointerdown", onPointerDownOutside)
    })
}

/** Flips a chevron through 90deg while switching between open/close states. */
export function flipDropdownChevron(el: HTMLElement | null, targetDeg: number) {
    if (!el) return
    el.getAnimations().forEach((animation) => animation.cancel())
    const fromDeg = targetDeg === 180 ? 0 : 180
    el.animate(
        [
            { transform: `rotateX(${fromDeg}deg)`, opacity: "1" },
            { transform: "rotateX(90deg)", opacity: "0.7" },
            { transform: `rotateX(${targetDeg}deg)`, opacity: "1" },
        ],
        { duration: CHEVRON_FLIP_DURATION, easing: "ease-in-out" },
    ).onfinish = () => {
        el.style.transform = `rotateX(${targetDeg}deg)`
    }
}
