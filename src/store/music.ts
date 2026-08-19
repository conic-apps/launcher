// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { convertFileSrc } from "@tauri-apps/api/core"
import { listMusicFiles, type MusicFile } from "@conic/music"
import { useConfigStore } from "@/store/config"
import { defineStore } from "pinia"
import { watch } from "vue"
import { window as appWindow } from "@tauri-apps/api"
import { Event } from "@tauri-apps/api/event"

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
let gainNode: GainNode | null = null
let targetFftSize = 2048

let lastPersistedAt = 0
const PERSIST_INTERVAL_MS = 5000

let volumeInitialized = false

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

/**
 * Routes the audio element through an analyser and a volume gain node so
 * visualizers can read live data and the volume can be controlled. The gain
 * node is required because once the element is pulled into the Web Audio
 * graph, its `volume` property is ignored by the webview.
 */
function ensureAnalyser(): AnalyserNode {
    if (!analyserNode) {
        const context = getAudioContext()
        analyserNode = context.createAnalyser()
        analyserNode.fftSize = targetFftSize
        analyserNode.smoothingTimeConstant = 0.8
        gainNode = context.createGain()
        gainNode.gain.value = getAudioElement().volume
        const source = context.createMediaElementSource(getAudioElement())
        source.connect(analyserNode)
        analyserNode.connect(gainNode)
        gainNode.connect(context.destination)
    }
    return analyserNode
}

/** Returns the analyser node feeding the currently playing audio, or null if not set up. */
export function getAnalyser(): AnalyserNode | null {
    return analyserNode
}

/**
 * Adjusts the FFT size so frequency resolution can scale with the visualizer's
 * bar count. Takes effect on the next analysis; also applies when the analyser
 * is created later.
 */
export function setAnalyserFftSize(size: number) {
    targetFftSize = size
    if (analyserNode) {
        analyserNode.fftSize = size
    }
}

/** Returns the sample rate of the audio context driving playback. */
export function getAudioSampleRate(): number {
    return getAudioContext().sampleRate
}

const MEDIA_SESSION_SUPPORTED = typeof navigator !== "undefined" && "mediaSession" in navigator

let mediaSessionSetup = false

/**
 * Registers OS media control handlers via the Media Session API so the system
 * can control playback (play/pause/previous/next/seek) and show the track name.
 * No-op when the platform webview lacks the API.
 */
function setupMediaSession() {
    if (!MEDIA_SESSION_SUPPORTED || mediaSessionSetup) {
        return
    }
    mediaSessionSetup = true
    const session = navigator.mediaSession
    const actions: [MediaSessionAction, MediaSessionActionHandler][] = [
        [
            "play",
            () => {
                void useMusicStore().resume()
            },
        ],
        [
            "pause",
            () => {
                useMusicStore().pause()
            },
        ],
        [
            "previoustrack",
            () => {
                void useMusicStore().prev()
            },
        ],
        [
            "nexttrack",
            () => {
                void useMusicStore().next()
            },
        ],
        [
            "seekto",
            (details) => {
                const store = useMusicStore()
                if (typeof details.seekTime === "number") {
                    store.seek(details.seekTime)
                }
            },
        ],
    ]
    for (const [action, handler] of actions) {
        try {
            session.setActionHandler(action, handler)
        } catch {
            // unsupported action on this platform; ignore
        }
    }
}

/** Reports the current track name to the OS media controls. */
function updateMediaSessionMetadata() {
    if (!MEDIA_SESSION_SUPPORTED) {
        return
    }
    const track = useMusicStore().currentTrack
    navigator.mediaSession.metadata = track ? new MediaMetadata({ title: track.name }) : null
}

/** Syncs the OS media controls playback state. */
function setMediaSessionPlaybackState(state: MediaSessionPlaybackState) {
    if (MEDIA_SESSION_SUPPORTED) {
        navigator.mediaSession.playbackState = state
    }
}

export const useMusicStore = defineStore("music", {
    state: () => ({
        tracks: [] as MusicFile[],
        currentIndex: -1 as number,
        isPlaying: false,
        shuffle: false,
        repeat: false,
        currentTime: 0,
        duration: 0,
        loading: false,
        error: null as string | null,
        panelOpen: false,
        backgrounded: false,
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

        /**
         * Applies the configured volume to the audio output, using the
         * background volume when the window is not focused. When `smooth` is
         * set, the change is ramped over 1s instead of jumping instantly.
         */
        applyVolume(smooth = false) {
            const config = useConfigStore()
            const percent = this.backgrounded
                ? config.music.main_volumn_background
                : config.music.main_volumn
            const volume = Math.min(Math.max(percent / 100, 0), 1)
            const audio = getAudioElement()
            audio.volume = volume
            if (gainNode) {
                const gain = gainNode.gain
                const now = getAudioContext().currentTime
                if (smooth) {
                    gain.cancelScheduledValues(now)
                    gain.setValueAtTime(gain.value, now)
                    gain.linearRampToValueAtTime(volume, now + 1.0)
                } else {
                    gain.setValueAtTime(volume, now)
                }
            }
        },

        /**
         * Registers window focus tracking so the volume switches between the
         * main and background settings, and watches those settings for changes.
         */
        init() {
            if (volumeInitialized) {
                return
            }
            volumeInitialized = true
            setupMediaSession()
            appWindow
                .getCurrentWindow()
                .isFocused()
                .then((focused) => {
                    this.backgrounded = !focused
                    this.applyVolume()
                })
            appWindow.getCurrentWindow().onFocusChanged((event: Event<boolean>) => {
                this.backgrounded = !event.payload
                this.applyVolume(true)
            })
            watch(
                () =>
                    [
                        useConfigStore().music.main_volumn,
                        useConfigStore().music.main_volumn_background,
                    ] as const,
                () => this.applyVolume(),
            )
        },

        /**
         * Loads the current track's source into the audio element and wires up
         * events, without starting playback.
         */
        async preparePlayback() {
            const track = this.currentTrack
            if (track === null) {
                return
            }
            const audio = getAudioElement()
            ensureAnalyser()
            await getAudioContext().resume()
            audio.src = convertFileSrc(track.path) + "?t=" + Date.now()
            this.applyVolume()
            this.attachAudioEvents(audio)
            updateMediaSessionMetadata()
        },

        async startPlayback() {
            await this.preparePlayback()
            try {
                await getAudioElement().play()
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
         * Loads tracks and always restores the last playing track and its
         * position from localStorage, regardless of settings. Playback only
         * starts when both the background music and resume-on-startup settings
         * are enabled; otherwise the track stays paused. When the saved track
         * cannot be found, the first track is selected in a paused state.
         */
        async restoreSession() {
            await this.loadTracks()
            if (this.tracks.length === 0) {
                return
            }
            const saved = loadTrackState()
            const restoredIndex =
                saved !== null ? this.tracks.findIndex((track) => track.path === saved.path) : -1
            this.currentIndex = restoredIndex >= 0 ? restoredIndex : 0
            await this.preparePlayback()
            if (restoredIndex >= 0 && saved !== null && saved.currentTime > 0) {
                this.seek(saved.currentTime)
            }
            const config = useConfigStore()
            if (config.music.enabled && config.music.resume_on_startup && restoredIndex >= 0) {
                try {
                    await getAudioElement().play()
                    this.isPlaying = true
                } catch (error) {
                    console.error("Failed to play music", error)
                }
            }
        },

        async resume() {
            const audio = getAudioElement()
            if (!audio.src) {
                await this.preparePlayback()
            }
            ensureAnalyser()
            await getAudioContext().resume()
            this.applyVolume()
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
            this.repeat = !this.repeat
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
                setMediaSessionPlaybackState("paused")
            }
            audio.onplay = () => {
                this.isPlaying = true
                setMediaSessionPlaybackState("playing")
            }
            audio.onerror = () => {
                console.error("Audio playback error", audio.error)
            }
        },

        handleTrackEnded() {
            if (this.repeat) {
                this.seek(0)
                void this.resume()
                return
            }
            void this.next()
        },
    },
})
