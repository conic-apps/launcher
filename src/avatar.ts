// Amethyst Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

export async function getAvatar(src: string, size: number) {
    const canvas = document.createElement("canvas")
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext("2d")
    if (ctx == null) {
        return ""
    }
    const img = new Image()
    img.src = src
    await new Promise<void>((reslove) => {
        img.onload = function () {
            const scale = img.width / 64
            const faceOffset = Math.round(size / 18.0)
            ctx.imageSmoothingEnabled = false
            /* Inspired by HMCL */
            ctx.drawImage(
                img,
                8 * scale,
                8 * scale,
                16 * scale - 8 * scale,
                16 * scale - 8 * scale,
                faceOffset,
                faceOffset,
                size - faceOffset - faceOffset,
                size - faceOffset - faceOffset,
            )
            ctx.drawImage(
                img,
                40 * scale,
                8 * scale,
                48 * scale - 40 * scale,
                16 * scale - 8 * scale,
                0,
                0,
                size,
                size,
            )
            reslove()
        }
    })
    return canvas.toDataURL("image/png")
}
