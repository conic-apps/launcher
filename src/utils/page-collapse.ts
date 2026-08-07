// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// 页面物理坍塌彩蛋：把页面上指定的 UI 块临时交给 Matter.js 物理引擎，
// 产生重力掉落、旋转、碰撞、落地堆叠的效果，结束后完全恢复原有 DOM 样式。
//
// 用法：
//   import { collapsePage } from "@/utils/page-collapse"
//   collapsePage()                                        // 默认掉落当前页主要区块
//   collapsePage({ selector: ".instances-list, .tool-bar" })
//   collapsePage({ gravity: 1.4, restitution: 0.4, duration: 3500 })

import { Body, Bodies, Composite, Engine, Events, Runner, type Body as MatterBody } from "matter-js"

export interface CollapsePageOptions {
    /** 需要掉落的元素选择器，多个用逗号分隔。默认匹配当前页面的主要区块。 */
    selector?: string
    /** 限定在某个容器内查找元素，默认在整个 document 中查找。 */
    container?: ParentNode
    /** 重力强度（y 轴缩放），默认 1。 */
    gravity?: number
    /** 弹性系数，默认 0.6。 */
    restitution?: number
    /** 表面摩擦力，默认 0.4。 */
    friction?: number
    /** 空气阻力，默认 0.012。 */
    frictionAir?: number
    /** 效果总时长（毫秒），默认 2800。 */
    duration?: number
    /** 每个元素依次开始掉落的延迟（毫秒），默认 90，传 0 表示同时掉落。 */
    stagger?: number
    /** 结束时元素滑回原位的时间（毫秒），默认 260。 */
    restoreDuration?: number
    /** 地面 Y 坐标（视口坐标），默认在视口底部下方，让元素落在屏幕底缘。 */
    groundY?: number
    /** 动画结束（样式恢复完成）后的回调。 */
    onComplete?: () => void
}

interface CollapsedElement {
    el: HTMLElement
    body: MatterBody
    startX: number
    startY: number
    delay: number
    restingTransform: string
    savedStyle: string | null
}

const DEFAULT_SELECTOR = ".current-instance, .instances-list, .game-view-footer"

const DEFAULTS = {
    gravity: 1,
    restitution: 0.6,
    friction: 0.4,
    frictionAir: 0.012,
    duration: 2800,
    stagger: 90,
    restoreDuration: 260,
}

const COLLAPSE_Z_INDEX = "2147483000"

let active = false

function randomInRange(min: number, max: number) {
    return min + Math.random() * (max - min)
}

/**
 * 判断元素自身或任一祖先带有 data-no-collapse 属性。
 * 带该属性的元素不参与物理模拟。
 */
function isExcluded(el: HTMLElement) {
    if (el.hasAttribute("data-no-collapse")) return true
    let node: Element | null = el.parentElement
    while (node) {
        if (node.hasAttribute("data-no-collapse")) return true
        node = node.parentElement
    }
    return false
}

/**
 * 查找会成为 position: fixed 后代包含块的最近祖先。
 * transform / perspective / filter / backdrop-filter / will-change /
 * contain 都会把 fixed 后代的定位基准从视口改为该元素。
 * 返回该包含块左上角（padding box）的视口坐标，用于把元素换算成 fixed 后不跳动。
 */
function getFixedContainingBlock(el: Element) {
    let node: Element | null = el.parentElement
    while (node && node !== document.body && node !== document.documentElement) {
        const style = getComputedStyle(node)
        const willChange = style.willChange
        if (
            style.transform !== "none" ||
            style.perspective !== "none" ||
            style.filter !== "none" ||
            style.backdropFilter !== "none" ||
            /transform|perspective|filter|backdrop-filter/.test(willChange) ||
            /layout|paint|strict|content/.test(style.contain)
        ) {
            const rect = node.getBoundingClientRect()
            return {
                offsetX: rect.left + (node.clientLeft || 0),
                offsetY: rect.top + (node.clientTop || 0),
            }
        }
        node = node.parentElement
    }
    return { offsetX: 0, offsetY: 0 }
}

/**
 * 让页面 UI 元素失去重力支撑、整块掉落的物理彩蛋。
 * 仅临时改写 DOM 的 inline style，不触碰 Vue 状态与组件结构，结束后完全恢复。
 * 已在运行期间重复调用会被忽略。
 */
export function collapsePage(options: CollapsePageOptions = {}): void {
    if (active || typeof window === "undefined" || typeof document === "undefined") return

    const selector = options.selector ?? DEFAULT_SELECTOR
    const container = options.container ?? document
    const gravity = options.gravity ?? DEFAULTS.gravity
    const restitution = options.restitution ?? DEFAULTS.restitution
    const friction = options.friction ?? DEFAULTS.friction
    const frictionAir = options.frictionAir ?? DEFAULTS.frictionAir
    const duration = options.duration ?? DEFAULTS.duration
    const stagger = options.stagger ?? DEFAULTS.stagger
    const restoreDuration = options.restoreDuration ?? DEFAULTS.restoreDuration

    const matches = container.querySelectorAll<HTMLElement>(selector)

    const elements: HTMLElement[] = []
    for (const el of matches) {
        if (isExcluded(el)) continue
        if (elements.some((other) => other.contains(el))) continue
        elements.push(el)
    }
    if (elements.length === 0) return

    active = true

    const engine = Engine.create()
    engine.gravity.y = gravity

    const viewportW = window.innerWidth
    const viewportH = window.innerHeight
    const groundY = options.groundY ?? viewportH + 24

    const collapsed: CollapsedElement[] = []

    elements.forEach((el, index) => {
        const rect = el.getBoundingClientRect()
        if (rect.width === 0 || rect.height === 0) return

        const cb = getFixedContainingBlock(el)
        const baseX = rect.left - cb.offsetX
        const baseY = rect.top - cb.offsetY
        const centerX = rect.left + rect.width / 2
        const centerY = rect.top + rect.height / 2

        const savedStyle = el.getAttribute("style")
        const restingTransform = getComputedStyle(el).transform

        const body = Bodies.rectangle(centerX, centerY, rect.width, rect.height, {
            restitution,
            friction,
            frictionAir,
            angle: randomInRange(-0.06, 0.06),
        })
        Body.setVelocity(body, { x: randomInRange(-1.5, 1.5), y: randomInRange(-1, 1) })
        Body.setAngularVelocity(body, randomInRange(-0.06, 0.06))

        el.style.transition = "none"
        el.style.position = "fixed"
        el.style.left = `${baseX}px`
        el.style.top = `${baseY}px`
        el.style.width = `${rect.width}px`
        el.style.height = `${rect.height}px`
        el.style.margin = "0"
        el.style.transform = "none"
        el.style.willChange = "transform"
        el.style.zIndex = COLLAPSE_Z_INDEX
        el.style.pointerEvents = "none"

        collapsed.push({
            el,
            body,
            startX: centerX,
            startY: centerY,
            delay: index * stagger,
            restingTransform,
            savedStyle,
        })

        Composite.add(engine.world, body)
    })

    if (collapsed.length === 0) {
        active = false
        return
    }

    const wallOptions = { isStatic: true, restitution, friction }
    const wallThickness = 200
    const leftWall = Bodies.rectangle(
        -wallThickness / 2 - 60,
        viewportH / 2,
        wallThickness,
        viewportH * 2 + wallThickness,
        wallOptions,
    )
    const rightWall = Bodies.rectangle(
        viewportW + wallThickness / 2 + 60,
        viewportH / 2,
        wallThickness,
        viewportH * 2 + wallThickness,
        wallOptions,
    )
    const ground = Bodies.rectangle(
        viewportW / 2,
        groundY,
        viewportW + wallThickness * 2,
        wallThickness,
        wallOptions,
    )
    Composite.add(engine.world, [leftWall, rightWall, ground])

    const runner = Runner.create()
    Runner.run(runner, engine)

    const startTime = performance.now()
    let ended = false

    function sync() {
        const elapsed = performance.now() - startTime
        for (const item of collapsed) {
            if (elapsed < item.delay) {
                Body.setPosition(item.body, { x: item.startX, y: item.startY })
                Body.setVelocity(item.body, { x: 0, y: 0 })
                Body.setAngularVelocity(item.body, 0)
                item.el.style.transform = "none"
                continue
            }
            const dx = item.body.position.x - item.startX
            const dy = item.body.position.y - item.startY
            const angle = item.body.angle
            item.el.style.transform = `translate3d(${dx}px, ${dy}px, 0) rotate(${angle}rad)`
        }
        if (elapsed >= duration) beginRestore()
    }

    function beginRestore() {
        if (ended) return
        ended = true
        Events.off(engine, "afterUpdate", sync)
        for (const item of collapsed) {
            item.el.style.transition = `transform ${restoreDuration}ms ease`
            item.el.style.transform = item.restingTransform
        }
        if (restoreDuration > 0) {
            window.setTimeout(restore, restoreDuration)
        } else {
            restore()
        }
    }

    function restore() {
        Runner.stop(runner)
        Composite.clear(engine.world, false)
        Engine.clear(engine)
        for (const item of collapsed) {
            const el = item.el
            if (item.savedStyle == null) {
                el.removeAttribute("style")
            } else {
                el.setAttribute("style", item.savedStyle)
            }
        }
        active = false
        options.onComplete?.()
    }

    Events.on(engine, "afterUpdate", sync)
}
