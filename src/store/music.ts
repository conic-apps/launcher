// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { convertFileSrc } from "@tauri-apps/api/core"
import { listMusicFiles, type MusicFile } from "@conic/music"
import { useConfigStore } from "@/store/config"
import { defineStore } from "pinia"

export type RepeatMode = "off" | "all" | "one"

const REPEAT_MODES: RepeatMode[] = ["off", "all", "one"]

const SAVED_TRACK_KEY = "conic.music.lastTrack"

type SavedTrackState = {
    path: string
    currentTime: number
}

function saveTrackState(state: SavedTrackState) {
    try {
        localStorage.setItem(SAVED_TRACK_KEY, JSON.stringify(state))
    } catch {
        // localStorage may be unavailable; ignore
    }
}

function loadTrackState(): SavedTrackState | null {
    try {
        const raw = localStorage.getItem(SAVED_TRACK_KEY)
        if (!raw) {
            return null
        }
        const parsed = JSON.parse(raw) as Partial<SavedTrackState>
        if (typeof parsed.path !== "string") {
            return null
        }
        return {
            path: parsed.path,
            currentTime: typeof parsed.currentTime === "number" ? parsed.currentTime : 0,
        }
    } catch {
        return null
    }
}

let audioElement: HTMLAudioElement | null = null
let audioContext: AudioContext | null = null
let analyserNode: AnalyserNode | null = null

let lastPersistedAt = 0
const PERSIST_INTERVAL_MS = 5000

function getAudioElement(): HTMLAudioElement {
    if (!audioElement) {
        audioElement = new Audio()
        audioElement.preload = "auto"
    }
    return audioElement
}

function getAudioContext(): AudioContext {
    if (!audioContext) {
        audioContext = new AudioContext()
    }
    return audioContext
}

/** Routes the audio element through an analyser so visualizers can read live data. */
function ensureAnalyser(): AnalyserNode {
    if (!analyserNode) {
        const context = getAudioContext()
        analyserNode = context.createAnalyser()
        analyserNode.fftSize = 1024
        analyserNode.smoothingTimeConstant = 0.8
        const source = context.createMediaElementSource(getAudioElement())
        source.connect(analyserNode)
        analyserNode.connect(context.destination)
    }
    return analyserNode
}

/** Returns the analyser node feeding the currently playing audio, or null if not set up. */
export function getAnalyser(): AnalyserNode | null {
    return analyserNode
}

/** Returns the sample rate of the audio context driving playback. */
export function getAudioSampleRate(): number {
    return getAudioContext().sampleRate
}

export const useMusicStore = defineStore("music", {
    state: () => ({
        tracks: [] as MusicFile[],
        currentIndex: -1 as number,
        isPlaying: false,
        shuffle: false,
        repeatMode: "off" as RepeatMode,
        currentTime: 0,
        duration: 0,
        loading: false,
        error: null as string | null,
        panelOpen: false,
    }),
    getters: {
        currentTrack(state): MusicFile | null {
            return state.currentIndex >= 0 && state.currentIndex < state.tracks.length
                ? state.tracks[state.currentIndex]
                : null
        },
        progress(state): number {
            if (state.duration <= 0) {
                return 0
            }
            return state.currentTime / state.duration
        },
    },
    actions: {
        async loadTracks() {
            this.loading = true
            try {
                this.tracks = await listMusicFiles()
            } catch (error) {
                console.error("Failed to load music files", error)
                this.tracks = []
            } finally {
                this.loading = false
            }
        },

        async playIndex(index: number) {
            if (index < 0 || index >= this.tracks.length) {
                return
            }
            this.currentIndex = index
            await this.startPlayback()
            this.persistState()
        },

        async togglePlay() {
            if (this.currentTrack === null) {
                if (this.tracks.length > 0) {
                    await this.playIndex(0)
                }
                return
            }
            if (this.isPlaying) {
                this.pause()
            } else {
                await this.resume()
            }
        },

        async startPlayback() {
            const track = this.currentTrack
            if (track === null) {
                return
            }
            const audio = getAudioElement()
            ensureAnalyser()
            await getAudioContext().resume()
            audio.src = convertFileSrc(track.path)
            audio.volume = 1
            this.attachAudioEvents(audio)
            try {
                await audio.play()
                this.isPlaying = true
            } catch (error) {
                console.error("Failed to play music", error)
            }
        },

        /** Persists the current track and playback position to localStorage. */
        persistState() {
            const track = this.currentTrack
            if (track === null) {
                return
            }
            saveTrackState({
                path: track.path,
                currentTime: this.currentTime,
            })
        },

        /**
         * Loads tracks and resumes the last playing track and its position from
         * localStorage, gated by the background music and resume settings.
         */
        async restoreSession() {
            await this.loadTracks()
            const config = useConfigStore()
            if (!config.music.enabled || !config.music.resume_on_startup) {
                return
            }
            const saved = loadTrackState()
            if (saved === null) {
                return
            }
            const index = this.tracks.findIndex((track) => track.path === saved.path)
            if (index < 0) {
                return
            }
            this.currentIndex = index
            await this.startPlayback()
            if (saved.currentTime > 0) {
                this.seek(saved.currentTime)
            }
        },

        async resume() {
            const audio = getAudioElement()
            ensureAnalyser()
            await getAudioContext().resume()
            try {
                await audio.play()
                this.isPlaying = true
            } catch (error) {
                console.error("Failed to resume music", error)
            }
        },

        pause() {
            getAudioElement().pause()
            this.isPlaying = false
            this.persistState()
        },

        async next() {
            if (this.tracks.length === 0) {
                return
            }
            if (this.shuffle) {
                const nextIndex = this.randomIndex()
                await this.playIndex(nextIndex)
                return
            }
            const nextIndex = (this.currentIndex + 1) % this.tracks.length
            await this.playIndex(nextIndex)
        },

        async prev() {
            if (this.tracks.length === 0) {
                return
            }
            if (this.currentTime > 3) {
                this.seek(0)
                return
            }
            const prevIndex = (this.currentIndex - 1 + this.tracks.length) % this.tracks.length
            await this.playIndex(prevIndex)
        },

        seek(time: number) {
            const audio = getAudioElement()
            audio.currentTime = time
            this.currentTime = time
        },

        seekRatio(ratio: number) {
            this.seek(ratio * this.duration)
        },

        toggleShuffle() {
            this.shuffle = !this.shuffle
        },

        cycleRepeat() {
            const currentModeIndex = REPEAT_MODES.indexOf(this.repeatMode)
            this.repeatMode = REPEAT_MODES[(currentModeIndex + 1) % REPEAT_MODES.length]
        },

        randomIndex(): number {
            if (this.tracks.length <= 1) {
                return 0
            }
            let randomIndex = this.currentIndex
            while (randomIndex === this.currentIndex) {
                randomIndex = Math.floor(Math.random() * this.tracks.length)
            }
            return randomIndex
        },

        openPanel() {
            this.panelOpen = true
        },

        closePanel() {
            this.panelOpen = false
        },

        togglePanel() {
            this.panelOpen = !this.panelOpen
        },

        attachAudioEvents(audio: HTMLAudioElement) {
            audio.onloadedmetadata = () => {
                this.duration = audio.duration || 0
            }
            audio.ontimeupdate = () => {
                this.currentTime = audio.currentTime
                const now = Date.now()
                if (now - lastPersistedAt >= PERSIST_INTERVAL_MS) {
                    lastPersistedAt = now
                    this.persistState()
                }
            }
            audio.onended = () => {
                this.handleTrackEnded()
            }
            audio.onpause = () => {
                this.isPlaying = false
                this.persistState()
            }
            audio.onplay = () => {
                this.isPlaying = true
            }
            audio.onerror = () => {
                console.error("Audio playback error", audio.error)
            }
        },

        handleTrackEnded() {
            if (this.repeatMode === "one") {
                this.seek(0)
                void this.resume()
                return
            }
            if (this.repeatMode === "all" || this.currentIndex < this.tracks.length - 1) {
                void this.next()
                return
            }
            this.isPlaying = false
        },
    },
})
