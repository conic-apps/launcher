<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — um launcher de Minecraft moderno e multiplataforma, mostrando quatro temas Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Um launcher de Minecraft pequeno, rápido e ágil</strong><br>
  Gerenciamento de múltiplas instâncias · Marketplace de mods e pacotes de recursos · Multijogador cross-LAN · Suporte a múltiplos temas</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md"><strong>Português (Brasil)</strong></a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## O que é isso

Conic Launcher é um launcher de Minecraft moderno para desktop: a lógica principal é implementada em Rust, a interface é construída com Tauri 2 + Vue 3. Possui uma pequena footprint de instalação, inicialização rápida e baixo consumo de recursos, até ajudando a reduzir as emissões de carbono.

Desde criar instâncias e instalar loaders até pesquisar mods e convidar amigos para jogar juntos, iniciar o jogo e rastrear o tempo de jogo — todo o fluxo de trabalho pode ser concluído em um único aplicativo.

> O projeto está atualmente em uma fase alfa inicial. As funcionalidades e a interface estão evoluindo rapidamente — sinta-se livre para experimentar e reportar problemas.

## Funcionalidades

### Gerenciamento de múltiplas instâncias

Navegue pelas instâncias agrupadas por loader de mods, com ordenação, pesquisa e favoritos. Cada instância possui suas próprias configurações, fundo personalizado, rastreamento de tempo de jogo e registro da última execução.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Suporte para todos os principais loaders

Escolha sua versão do Minecraft e versão do loader — o launcher cuida do resto. Vanilla, Forge, NeoForge, Fabric e Quilt são todos suportados, com progresso de instalação em tempo real.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Marketplace de mods e pacotes de recursos

Pesquisa integrada do Modrinth e CurseForge: navegue pelos detalhes do projeto e dependências, leia resumos traduzidos e instale mods ou pacotes de recursos em suas instâncias com um único clique. Você também pode gerenciar mods instalados localmente diretamente.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Multijogador cross-LAN Conic Nexus

Nenhuma ferramenta adicional necessária: crie um grupo, compartilhe o código de convite com seus amigos e entre no mesmo mundo juntos. O launcher detecta seu ambiente de rede (tipo NAT) e fornece sugestões de melhoria.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Múltiplos métodos de login

Suporta login com conta Microsoft (fluxo de código de dispositivo), login offline e qualquer servidor de autenticação externo Authlib-Injector / Yggdrasil. As contas oferecem visualização 3D de skins, exibição de capas e upload de skins.

<img width="840" height="533" alt="Captura de tela 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Experiência de inicialização tranquila

Varredura automática de Java no seu sistema, ou download automático de um runtime adequado. A memória é alocada automaticamente (com proteção de limite para Java de 32 bits). O processo de inicialização é totalmente visualizado: download de arquivos do jogo, instalação de loaders, resolução de dependências e espera para o início do processo.

Usuários avançados não são esquecidos — o garbage collector JVM, argumentos JVM, argumentos do jogo, classpath, comando wrapper e comandos antes/depois da inicialização são todos personalizáveis.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Saves e dados do jogo

Navegue pelos mapas de mundos salvos, gerencie pacotes de dados e pacotes de recursos, e visualize capturas de tela do jogo — tudo dentro do launcher.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interface e personalização

Quatro temas Catppuccin (Mocha · Macchiato · Frappé · Latte), além de variantes de alto contraste, com alternância automática claro/escuro seguindo as configurações do seu sistema. Fundos personalizados do launcher suportados. Reprodutor de música integrado com reprodução de música local e visualização de espectro.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### E mais

- Atualizações automáticas integradas e notificações de atualização de versões do jogo
- Downloads multi-thread com limites de conexões e velocidade ajustáveis; suporte a servidores espelho e proxy do sistema
- Pesquisa global (`/`) e paleta de comandos (`;`)
- Rastreamento de tempo de jogo e calendário de atividades

## Download

Baixe o instalador para sua plataforma no [GitHub Releases](https://github.com/conic-apps/launcher/releases):

| Plataforma | Arquitetura             | Formatos                       |
| ---------- | ----------------------- | ------------------------------ |
| Windows    | x64 · arm64             | MSI · instalador NSIS · exe portátil |
| macOS      | Apple Silicon · Intel   | DMG                            |
| Linux      | x64 · arm64             | deb · rpm · AppImage           |

O aplicativo inclui atualizações automáticas — não são necessárias atualizações manuais após a instalação.

## Compilar a partir do código-fonte

Certifique-se de ter [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) e [pnpm](https://pnpm.io/) instalados, e siga os [pré-requisitos do Tauri](https://tauri.app/start/prerequisites/) para sua plataforma.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Desenvolvimento e depuração
pnpm tauri build    # Build de produção
```

## Contribuir

Issues e Pull Requests (direcionados à branch `dev`) são bem-vindos. Antes de enviar, por favor execute:

```bash
pnpm check
```

## Arquitetura

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Diagrama de arquitetura: o frontend Vue 3 se comunica com o core Rust via Tauri IPC, com módulos crates separados por domínio abaixo">
</div>

O frontend (Vue 3 + Pinia) se comunica com as funcionalidades Rust via Tauri IPC. O `core` cuida do montagem da aplicação Tauri, enquanto as funcionalidades específicas são divididas em módulos `crates/*` do workspace, organizados por domínio, que evoluem independentemente e se combinam conforme necessário.

## Licença

Este projeto é distribuído sob [GPL-3.0](../../LICENSE) com termos adicionais sob a seção 7 do GPLv3:

1. Ao distribuir versões modificadas, você deve alterar razoavelmente o nome do software ou o número da versão para distingui-lo do original (de acordo com [GPL-3.0 §7(c)](../../LICENSE)); você precisa substituir todas as referências ao nome do projeto no código-fonte.
2. Você não deve remover os avisos de copyright exibidos no software (de acordo com [GPL-3.0 §7(b)](../../LICENSE)).
3. Se alguém entrar em termos contratuais com os destinatários e assumir responsabilidade, o licenciante e os autores não são solidariamente responsáveis (de acordo com [GPL-3.0 §7(b)](../../LICENSE)).

## Agradecimentos

- [Catppuccin](https://catppuccin.com) — Bela paleta de temas em cores pastéis
- [BMCLAPI](https://bmclapi2.bangbang93.com/) e [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) por bangbang93 — Aceleração de downloads e bootstrap da instalação do Forge
- [MCIM](https://mod.mcimirror.top) — Resumos traduzidos do CurseForge e cache espelho
- Todos os jogadores que forneceram sugestões e feedback

## Aviso legal

Conic Launcher não é um produto oficial do Minecraft e não é aprovado ou afiliado à Mojang Studios. "Minecraft" é uma marca registrada da Mojang AB. Qualquer uso da marca Minecraft neste projeto está em conformidade com as <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Diretrizes de Marca e Ativos</a> da Mojang Studios.
