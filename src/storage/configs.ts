export { };
// import { invoke } from "@tauri-apps/api";
// import { defineStore } from "pinia";

// const defaultConfig = {
//     general: {
//         update: {
//             beta: true,
//             automaticUpdate: true,
//             extension: true,
//         }
//     },
//     game: {
//         jvm: {
//             autoSelectJVM: true,
//         },
//         launch: {
//             gameWindowTitle: '',
//             launcherName: '',
//             serverIP: '',
//             processPriority: 0, // 0 highest 4 lowest
//             width: '',
//             height: '',
//             autoAllocateMemory: true,
//             memory: 2048
//         },
//         advance: {
//             gameArgs: '',
//             execBeforeLaunch: '',
//             wrapCommand: '',
//             execAfterLaunch: '',
//             JVMArgs: '',
//             permanentMemoryArea: ''
//         },
//         debug: {
//             lwjgl: '',
//             dontCheckJVMArgs: false,
//             dontCheckResourceFile: false,
//             dontCheckJVMCompatibility: false,
//             useSystemGLFW: false,
//             useSystemOpenAL: false
//         }
//     },
//     advance: {
//         game: {
//             checkLibraries: true,
//             garbageCollector: 0
//         },
//         debug: {
//             saveLauncherLogs: false,
//         }
//     },
//     download: {
//         throttling: false,
//         maxConcurrency: 256,
//         maxSpeed: 50,
//         source: 0,
//         customSource: ''
//     },
//     accessibility: {
//         releaseUpdateReminder: false,
//         snapshotUpdateReminder: false,
//         useSystemLang: true,
//         animationSpeed: 2,
//         disableAllAnimations: false,
//     },
//     extend: {
//         enableAll: false
//     }
// }

// const userConfig = await invoke('get-user-config') as object

// export const useConfigStore = defineStore('configs', {
//     state: () => {
//         return {
//             ...defaultConfig,
//             ...userConfig,
//         }
//     },
//     actions: {
//         get(key:string) {
//             const keys = key.split('.')
//             let result
//             for (let index = 0; index < keys.length; index++) {
//                 const key = keys[index];
//                 if (index === 0) {
//                     result = a
//                     continue
//                 }
//                 result = result[key]
//                 if (typeof result === 'undefined') {
//                     break
//                 }
//             }
//             return result
//         },
//         set() {

//         }
//     }
// })

