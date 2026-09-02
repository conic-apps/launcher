<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — un lanzador de Minecraft moderno y multiplataforma que muestra cuatro temas Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Un lanzador de Minecraft pequeño, rápido y ágil</strong><br>
  Gestión de múltiples instancias · Marketplace de mods y paquetes de recursos · Multijugador cross-LAN · Soporte multi-tema</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md"><strong>Español</strong></a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Qué es esto

Conic Launcher es un lanzador de Minecraft moderno para escritorio: la lógica principal está implementada en Rust, la interfaz está construida con Tauri 2 + Vue 3. Tiene una huella de instalación pequeña, inicio rápido y bajo consumo de recursos, incluso ayudando a reducir las emisiones de carbono.

Desde crear instancias e instaladores, hasta buscar mods e invitar a amigos para jugar juntos, lanzar el juego y rastrear el tiempo de juego — todo el flujo de trabajo se puede completar en una sola aplicación.

> El proyecto se encuentra actualmente en una fase alpha temprana. Las funciones y la interfaz están evolucionando rápidamente — siéntete libre de probarlo y reportar problemas.

## Características

### Gestión de múltiples instancias

Explora instancias agrupadas por cargador de mods, con ordenación, búsqueda y favoritos. Cada instancia tiene su propia configuración, fondo personalizado, seguimiento de tiempo de juego y marca de tiempo de la última ejecución.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Soporte para todos los cargadores principales

Elige tu versión de Minecraft y versión del cargador — el lanzador se encarga del resto. Vanilla, Forge, NeoForge, Fabric y Quilt son todos compatibles, con progreso de instalación en tiempo real.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Marketplace de mods y paquetes de recursos

Búsqueda integrada de Modrinth y CurseForge: explora detalles del proyecto y dependencias, lee resúmenes traducidos e instala mods o paquetes de recursos en tus instancias con un solo clic. También puedes gestionar mods instalados localmente directamente.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Multijugador cross-LAN Conic Nexus

No se necesitan herramientas adicionales: crea un grupo, comparte el código de invitación con tus amigos y únete al mismo mundo juntos. El lanzador detecta tu entorno de red (tipo NAT) y ofrece sugerencias de mejora.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Múltiples métodos de inicio de sesión

Soporta inicio de sesión con cuenta Microsoft (flujo de código de dispositivo), inicio de sesión sin conexión y cualquier servidor de autenticación externo Authlib-Injector / Yggdrasil. Las cuentas ofrecen vista previa 3D de skins, visualización de capas y carga de skins.

<img width="840" height="533" alt="Captura de pantalla 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Experiencia de lanzamiento sin complicaciones

Escaneo automático de Java en tu sistema, o descarga automática de un runtime adecuado. La memoria se asigna automáticamente (con protección de límite de Java de 32 bits). El proceso de lanzamiento se visualiza completamente: descarga de archivos del juego, instalación de cargadores, resolución de dependencias y espera del inicio del proceso.

Los usuarios avanzados no son olvidados — el recolector de basura JVM, argumentos JVM, argumentos del juego, classpath, comando wrapper y comandos pre/post lanzamiento son todos personalizables.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Guardados y datos del juego

Explora mapas de mundos guardados, gestiona paquetes de datos y paquetes de recursos, y ve capturas de pantalla del juego — todo desde el lanzador.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interfaz y personalización

Cuatro temas Catppuccin (Mocha · Macchiato · Frappé · Latte), más variantes de alto contraste, con cambio automático claro/oscuro según la configuración de tu sistema. Fondos personalizados del lanzador compatibles. Reproductor de música integrado con reproducción de música local y visualización de espectro.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### Y más

- Actualizaciones automáticas integradas y notificaciones de actualización de versiones del juego
- Descargas multi-hilo con límites de conexiones y velocidad ajustables; soporte de servidores espejo y proxy del sistema
- Búsqueda global (`/`) y paleta de comandos (`;`)
- Seguimiento de tiempo de juego y calendario de actividad

## Descarga

Descarga el instalador para tu plataforma desde [GitHub Releases](https://github.com/conic-apps/launcher/releases):

| Plataforma | Arquitectura           | Formatos                       |
| ---------- | ---------------------- | ------------------------------ |
| Windows    | x64 · arm64            | MSI · instalador NSIS · exe portátil |
| macOS      | Apple Silicon · Intel  | DMG                            |
| Linux      | x64 · arm64            | deb · rpm · AppImage           |

La aplicación incluye actualizaciones automáticas — no se necesitan actualizaciones manuales después de la instalación.

## Compilar desde el código fuente

Asegúrate de tener [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) y [pnpm](https://pnpm.io/) instalados, y sigue los [prerrequisitos de Tauri](https://tauri.app/start/prerequisites/) para tu plataforma.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Desarrollo y depuración
pnpm tauri build    # Build de producción
```

## Contribuir

Los Issues y Pull Requests (dirigidos a la rama `dev`) son bienvenidos. Antes de enviar, por favor ejecuta:

```bash
pnpm check
```

## Arquitectura

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Diagrama de arquitectura: el frontend Vue 3 se comunica con el núcleo Rust a través de Tauri IPC, con módulos crates separados por dominio debajo">
</div>

El frontend (Vue 3 + Pinia) se comunica con las funcionalidades de Rust a través de Tauri IPC. El `core` se encarga del ensamblaje de la aplicación Tauri, mientras que las funcionalidades específicas están divididas en módulos `crates/*` del workspace, organizados por dominio, que evolucionan independientemente y se combinan según sea necesario.

## Licencia

Este proyecto se distribuye bajo [GPL-3.0](../../LICENSE) con términos adicionales bajo la sección 7 del GPLv3:

1. Al distribuir versiones modificadas, debes cambiar razonablemente el nombre del software o el número de versión para distinguirlo del original (de acuerdo con [GPL-3.0 §7(c)](../../LICENSE)); debes reemplazar todas las referencias al nombre del proyecto en el código fuente.
2. No debes eliminar los avisos de copyright que se muestran en el software (de acuerdo con [GPL-3.0 §7(b)](../../LICENSE)).
3. Si alguien entra en términos contractuales con los destinatarios y asume la responsabilidad, el licenciante y los autores no son solidariamente responsables (de acuerdo con [GPL-3.0 §7(b)](../../LICENSE)).

## Agradecimientos

- [Catppuccin](https://catppuccin.com) — Hermosa paleta de temas en colores pastel
- [BMCLAPI](https://bmclapi2.bangbang93.com/) y [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) por bangbang93 — Aceleración de descargas y arranque de instalación de Forge
- [MCIM](https://mod.mcimirror.top) — Resúmenes traducidos de CurseForge y caché de espejo
- Todos los jugadores que han dado sugerencias y retroalimentación

## Aviso legal

Conic Launcher no es un producto oficial de Minecraft y no está aprobado ni afiliado a Mojang Studios. "Minecraft" es una marca registrada de Mojang AB. Cualquier uso de la marca Minecraft en este proyecto cumple con las <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Directrices de marca y activos</a> de Mojang Studios.
