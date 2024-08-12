import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";

enum GarbageCollector {
    G1 = "G1",
    Serial = "Serial",
    Parallel = "Parallel",
    ParallelOld = "ParallelOld",
    Z = "Z",
}

const defaultConfig = {
    general: {
        update: {
            beta: true,
            automaticUpdate: true,
            extension: true,
        },
    },
    game: {
        jvm: {
            autoSelectJVM: true,
        },
        launch: {
            gameWindowTitle: "Minecraft ${mc_version}",
            launcherName: "AmethystLauncher",
            serverIP: null,
            processPriority: 2, // support 0-4, 0 highest 4 lowest 2 middle
            width: 854,
            height: 480,
            autoAllocateMemory: true,
            memory: 2048,
        },
        advance: {
            extraMinecraftArgs: null,
            extraJVMArgs: null,
            isDemo: false,
            execBeforeLaunch: null,
            wrapCommand: null,
            execAfterLaunch: null,
            permanentMemoryArea: null,
        },
        debug: {
            lwjglPath: null,
            renderer: "OpenGL", // can be OpenGL, Vulkan, Software, DirectX 12
            dontCheckJVMArgs: false,
            dontCheckResourceFile: false,
            dontCheckJVMCompatibility: false,
            //todo: dontAttemptNativeLib, false
            useSystemGLFW: false, // Linux Only
            useSystemOpenAL: false, // Linux Only
        },
    },
    advance: {
        game: {
            checkLibraries: true,
            garbageCollector: GarbageCollector,
        },
        debug: {
            saveLauncherLogs: false,
        },
    },
    download: {
        throttling: false,
        maxConcurrency: 256,
        maxSpeed: 50,
        source: 0,
        customSource: "",
    },
    accessibility: {
        releaseUpdateReminder: false,
        snapshotUpdateReminder: false,
        useSystemLang: true,
        animationSpeed: 2,
        disableAllAnimations: false,
    },
    extend: {
        enableAll: false,
    },
};

// alert(defaultConfig.advance.game.garbageCollector)

const userConfig = JSON.parse(await invoke("get_user_config")) as object;

export const useConfigStore = defineStore("configs", {
    state: () => {
        return {
            ...defaultConfig,
            ...userConfig,
        };
    },
    actions: {
        get(key: string) {
            const keys = key.split(".");
            let result;
            for (let index = 0; index < keys.length; index++) {
                const key = keys[index];
                if (index === 0) {
                    result = this.$state as any;
                    continue;
                }
                result = result[key];
                if (typeof result === "undefined") {
                    break;
                }
            }
            return result;
        },
        set() {
        },
    },
});
