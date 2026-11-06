<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — un lanzador de Minecraft moderno y multiplataforma que muestra cuatro temas Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Un lanzador de Minecraft pequeño, rápido y ágil</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md"><strong>Español</strong></a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Qué es esto

Conic Launcher es un lanzador de Minecraft de escritorio pequeño, rápido y ágil, implementado en Rust,
con una interfaz construida sobre [Slint](https://slint.dev). Instalación ligera, arranque rápido y
bajo consumo de recursos; incluso ayuda a reducir las emisiones de CO₂.

> El proyecto está en una fase Alpha temprana; las funciones y la interfaz evolucionan rápidamente.
> Pruébalo y háznos saber cualquier problema.

## Características

### Gestión de múltiples instancias

Explora las instancias agrupadas por cargador de mods, con ordenación, búsqueda y favoritos; cada
instancia puede tener sus propios ajustes especiales.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Soporte para todos los cargadores principales

Compatibilidad completa con Forge, NeoForge, Fabric y Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Marketplace de mods y paquetes de recursos

Búsqueda integrada de Modrinth y CurseForge: consulta los detalles de los proyectos y sus
dependencias dentro del lanzador, instala un mod o un paquete de recursos en una instancia con un
clic y gestiona directamente los mods instalados localmente.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Multijugador cross-LAN de Conic Nexus

No hacen falta herramientas adicionales: crea un grupo, envía el código de invitación a un amigo y
entrad juntos al mismo mundo. El lanzador detecta tu entorno de red (tipo de NAT) y sugiere mejoras.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Múltiples métodos de inicio de sesión

Compatible con el inicio de sesión de Microsoft (flujo de código de dispositivo), el inicio de sesión
sin conexión y cualquier servidor de autenticación externo Authlib-Injector / Yggdrasil.

<img width="840" height="533" alt="Captura de pantalla 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Experiencia de lanzamiento sin complicaciones

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Guardados y datos del juego

Explora los mapas de los mundos guardados, gestiona paquetes de datos y paquetes de recursos, y
consulta las capturas de pantalla del juego, todo dentro del lanzador.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interfaz y personalización

Cuatro temas de Catppuccin (Mocha · Macchiato · Frappé · Latte), además de variantes de alto contraste;
se admiten fondos propios y música de fondo para el lanzador.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Descarga

Descarga el instalador para tu plataforma desde
[GitHub Releases](https://github.com/conic-apps/launcher/releases) o la
[web oficial](https://conicmc.app):

| Plataforma | Arquitectura          | Formatos             |
| ---------- | --------------------- | -------------------- |
| Windows    | x64 · arm64           | MSI · exe portátil   |
| macOS      | Apple Silicon · Intel | DMG                  |
| Linux      | x64 · arm64           | deb · rpm · AppImage |

> Requiere macOS 11.0 (Big Sur) o posterior.

> Si usas la versión de archivo único en Windows, asegúrate de que exista un `VCRUNTIME140.dll`
> válido en el sistema.

## Compilar desde el código fuente

Asegúrate de tener instalado [Rust 1.88+](https://rustup.rs/); en Linux también necesitas las
[dependencias de compilación](../../CONTRIBUTING.md#linux-build-dependencies).

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Desarrollo y depuración
cargo build --release  # Compilación de producción
```

## Contribuir

¡Las contribuciones son bienvenidas! Lee [CONTRIBUTING.md](../../CONTRIBUTING.md) para saber cómo
empezar, configurar tu entorno de desarrollo y enviar un pull request.

## Licencia

Este proyecto se distribuye bajo [GPL-3.0](../../LICENSE), junto con todos los términos adicionales de
la sección 7 de la GPLv3.

## Agradecimientos

- [Catppuccin](https://catppuccin.com) — colores de tema suaves y bonitos
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) por
  bangbang93 — arranque de la instalación de Forge
- [MCIM](https://mod.mcimirror.top) — traducciones al chino de CurseForge y caché de espejo
- [HMCL](https://github.com/HMCL-dev/HMCL) — gestión de fallos
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — referencia para el análisis de mods
  locales

## Aviso legal

Conic Launcher no es un producto oficial de Minecraft y no está aprobado ni asociado con Mojang
Studios. «Minecraft» es una marca de Mojang AB. Cualquier uso de la marca Minecraft en este proyecto
cumple con las
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">directrices de marca y
activos</a> de Mojang Studios.
