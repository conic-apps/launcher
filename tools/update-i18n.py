#!/usr/bin/env python3
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

"""Add, re-key and drop entries in the Slint app's `.po` catalogues.

The catalogues were seeded by hand, and a few of the entries drifted away from
the `.slint` files they belong to — a component that was renamed left its
sentences under the old name, and a string that was added to a component nobody
re-extracted never made it in at all. At runtime a drifted entry is invisible:
`@tr` resolves under the *component's* name, so the lookup misses and the
sentence falls back to its English source, in every language.

So this takes the truth from two places and writes the catalogues back:

  * `slint-tr-extractor`'s `.pot`, for which `(context, msgid)` pairs the `.slint`
    files actually contain;
  * a table here, for what each of those pairs says in each of the twelve
    languages.

A pair that is in the `.pot` and has no translation below is reported and left
alone, rather than being written as an identity — a visible gap in the catalogue
is better than a sentence nobody will notice is missing.

    app: find ui -name '*.slint' | xargs slint-tr-extractor -o messages.pot
    python3 tools/update-i18n.py messages.pot
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

APP = Path(__file__).resolve().parent.parent
I18N = APP / "app" / "i18n"
DOMAIN = "conic-launcher"

# The languages, in the order the settings pickers list them.
LANGUAGES = [
    "zh_CN",
    "zh_TW",
    "ja_JP",
    "ko_KR",
    "de_DE",
    "fr_FR",
    "es_ES",
    "pt_BR",
    "ru_RU",
    "tr_TR",
    "pl_PL",
    "en_US",
]

# ---------------------------------------------------------------------------
# Re-keys: (old context, old msgid) -> new context. The sentences and their
# translations are already right; only the component they are looked up under
# moved.
# ---------------------------------------------------------------------------

# `ContentModsCurseforge`/`ContentModsModrinth`/… were folded into one
# `ContentCardGrid` with a `panels.slint` per panel, so the empty state each
# remote list draws now resolves under its own panel's name.
RECONTEXT = {
    ("RemoteListView", "Try adjusting keywords or filters and search again"): [
        "ContentModsPanel",
        "ContentResourcepacksPanel",
        "ContentPacksPanel",
    ],
    # The music player's strings were catalogued without a context, which means
    # they were only ever reachable by a component that did not exist.
    (None, "Shuffle"): ["MusicPlayerOverlay"],
    (None, "Repeat"): ["MusicPlayerOverlay"],
    (None, "Previous"): ["MusicPlayerOverlay"],
    (None, "Play"): ["MusicPlayerOverlay"],
    (None, "Pause"): ["MusicPlayerOverlay"],
    (None, "Next"): ["MusicPlayerOverlay"],
    (None, "Open music folder"): ["MusicPlayerOverlay"],
    (None, "Playlist"): ["MusicPlayerOverlay"],
    (None, "Nothing here"): ["MusicPlayerOverlay"],
}

# Sentences to drop: a component that no longer exists at all.
DROP = {
    ("RemoteListView", "Try adjusting keywords or filters and search again"),
    # The Data storage rows used to carry a "Change" button; the row itself is
    # the chooser now.
    ("SettingsGeneral", "Change"),
    # The Data storage rows' "Instances" label was clarified to say it holds the
    # game data too.
    ("SettingsGeneral", "Instances"),
    ("SetupStorage", "Instances"),
    # The add-account dialog's title shortened from a sentence to "Sign in".
    ("AccountAdd", "Choose an auth service and add an account"),
}

# ---------------------------------------------------------------------------
# New entries. `msgid` is the English source, which is also the `@tr()` argument.
# The keys are `(context, msgid)`.
# ---------------------------------------------------------------------------

SAVE_TAGS = {
    # The add-account dialog's title.
    "AccountAdd": {
        "Sign in": {
            "zh_CN": "登录",
            "zh_TW": "登入",
            "ja_JP": "ログイン",
            "ko_KR": "로그인",
            "de_DE": "Anmelden",
            "fr_FR": "Se connecter",
            "es_ES": "Iniciar sesión",
            "pt_BR": "Entrar",
            "ru_RU": "Войти",
            "tr_TR": "Giriş yap",
            "pl_PL": "Zaloguj się",
        },
    },
    # The wizard's storage step, and the label on the button that leaves it.
    "SetupStorage": {
        "Data storage": {
            "zh_CN": "数据存储",
            "zh_TW": "資料儲存",
            "ja_JP": "データの保存先",
            "ko_KR": "데이터 저장소",
            "de_DE": "Datenspeicher",
            "fr_FR": "Stockage des données",
            "es_ES": "Almacenamiento de datos",
            "pt_BR": "Armazenamento de dados",
            "ru_RU": "Хранилище данных",
            "tr_TR": "Veri depolama",
            "pl_PL": "Przechowywanie danych",
        },
        "Choose where Conic Launcher keeps its data. You can change this later in Settings.": {
            "zh_CN": "选择 Conic Launcher 保存数据的位置。你可以稍后在设置中更改。",
            "zh_TW": "選擇 Conic Launcher 儲存資料的位置。你可以稍後在設定中變更。",
            "ja_JP": "Conic Launcher がデータを保存する場所を選択します。後で設定から変更できます。",
            "ko_KR": "Conic Launcher가 데이터를 저장할 위치를 선택하세요. 나중에 설정에서 변경할 수 있습니다.",
            "de_DE": "Wähle, wo Conic Launcher seine Daten speichert. Du kannst das später in den Einstellungen ändern.",
            "fr_FR": "Choisissez où Conic Launcher conserve ses données. Vous pourrez modifier cela plus tard dans les paramètres.",
            "es_ES": "Elige dónde guarda Conic Launcher sus datos. Puedes cambiarlo más tarde en los ajustes.",
            "pt_BR": "Escolha onde o Conic Launcher guarda seus dados. Você pode alterar isso depois nas configurações.",
            "ru_RU": "Выберите, где Conic Launcher хранит свои данные. Это можно изменить позже в настройках.",
            "tr_TR": "Conic Launcher'ın verilerini nerede saklayacağını seçin. Bunu daha sonra Ayarlar'dan değiştirebilirsiniz.",
            "pl_PL": "Wybierz, gdzie Conic Launcher przechowuje swoje dane. Możesz to zmienić później w ustawieniach.",
        },
        "Launcher data": {
            "zh_CN": "启动器数据",
            "zh_TW": "啟動器資料",
            "ja_JP": "ランチャーのデータ",
            "ko_KR": "런처 데이터",
            "de_DE": "Launcher-Daten",
            "fr_FR": "Données du launcher",
            "es_ES": "Datos del launcher",
            "pt_BR": "Dados do launcher",
            "ru_RU": "Данные лаунчера",
            "tr_TR": "Başlatıcı verileri",
            "pl_PL": "Dane launchera",
        },
        "Minecraft install": {
            "zh_CN": "Minecraft 安装",
            "zh_TW": "Minecraft 安裝",
            "ja_JP": "Minecraft インストール",
            "ko_KR": "Minecraft 설치",
            "de_DE": "Minecraft-Installation",
            "fr_FR": "Installation de Minecraft",
            "es_ES": "Instalación de Minecraft",
            "pt_BR": "Instalação do Minecraft",
            "ru_RU": "Установка Minecraft",
            "tr_TR": "Minecraft kurulumu",
            "pl_PL": "Instalacja Minecraft",
        },
        "Instances and game data": {
            "zh_CN": "实例与游戏数据",
            "zh_TW": "實例與遊戲資料",
            "ja_JP": "インスタンスとゲームデータ",
            "ko_KR": "인스턴스 및 게임 데이터",
            "de_DE": "Instanzen und Spieldaten",
            "fr_FR": "Instances et données de jeu",
            "es_ES": "Instancias y datos del juego",
            "pt_BR": "Instâncias e dados do jogo",
            "ru_RU": "Сборки и игровые данные",
            "tr_TR": "Örnekler ve oyun verileri",
            "pl_PL": "Instancje i dane gry",
        },
    },
    "SetupView": {
        "Next (Color scheme)": {
            "zh_CN": "下一步（配色方案）",
            "zh_TW": "下一步（配色方案）",
            "ja_JP": "次へ（カラースキーム）",
            "ko_KR": "다음 (색 구성표)",
            "de_DE": "Weiter (Farbschema)",
            "fr_FR": "Suivant (Schéma de couleurs)",
            "es_ES": "Siguiente (Esquema de color)",
            "pt_BR": "Avançar (Esquema de cores)",
            "ru_RU": "Далее (Цветовая схема)",
            "tr_TR": "İleri (Renk şeması)",
            "pl_PL": "Dalej (Schemat kolorów)",
        },
    },
    # Settings → Data storage, and the restart prompt a change raises.
    "SettingsGeneral": {
        "Data storage": {
            "zh_CN": "数据存储",
            "zh_TW": "資料儲存",
            "ja_JP": "データの保存先",
            "ko_KR": "데이터 저장소",
            "de_DE": "Datenspeicher",
            "fr_FR": "Stockage des données",
            "es_ES": "Almacenamiento de datos",
            "pt_BR": "Armazenamento de dados",
            "ru_RU": "Хранилище данных",
            "tr_TR": "Veri depolama",
            "pl_PL": "Przechowywanie danych",
        },
        "Launcher data": {
            "zh_CN": "启动器数据",
            "zh_TW": "啟動器資料",
            "ja_JP": "ランチャーのデータ",
            "ko_KR": "런처 데이터",
            "de_DE": "Launcher-Daten",
            "fr_FR": "Données du launcher",
            "es_ES": "Datos del launcher",
            "pt_BR": "Dados do launcher",
            "ru_RU": "Данные лаунчера",
            "tr_TR": "Başlatıcı verileri",
            "pl_PL": "Dane launchera",
        },
        "Minecraft install": {
            "zh_CN": "Minecraft 安装",
            "zh_TW": "Minecraft 安裝",
            "ja_JP": "Minecraft インストール",
            "ko_KR": "Minecraft 설치",
            "de_DE": "Minecraft-Installation",
            "fr_FR": "Installation de Minecraft",
            "es_ES": "Instalación de Minecraft",
            "pt_BR": "Instalação do Minecraft",
            "ru_RU": "Установка Minecraft",
            "tr_TR": "Minecraft kurulumu",
            "pl_PL": "Instalacja Minecraft",
        },
        "Instances and game data": {
            "zh_CN": "实例与游戏数据",
            "zh_TW": "實例與遊戲資料",
            "ja_JP": "インスタンスとゲームデータ",
            "ko_KR": "인스턴스 및 게임 데이터",
            "de_DE": "Instanzen und Spieldaten",
            "fr_FR": "Instances et données de jeu",
            "es_ES": "Instancias y datos del juego",
            "pt_BR": "Instâncias e dados do jogo",
            "ru_RU": "Сборки и игровые данные",
            "tr_TR": "Örnekler ve oyun verileri",
            "pl_PL": "Instancje i dane gry",
        },
    },
    "RestartRequired": {
        "Restart required": {
            "zh_CN": "需要重启",
            "zh_TW": "需要重新啟動",
            "ja_JP": "再起動が必要です",
            "ko_KR": "재시작 필요",
            "de_DE": "Neustart erforderlich",
            "fr_FR": "Redémarrage requis",
            "es_ES": "Reinicio necesario",
            "pt_BR": "Reinicialização necessária",
            "ru_RU": "Требуется перезапуск",
            "tr_TR": "Yeniden başlatma gerekli",
            "pl_PL": "Wymagane ponowne uruchomienie",
        },
        "The launcher must restart before the new storage location is used.": {
            "zh_CN": "必须重启启动器才能使用新的存储位置。",
            "zh_TW": "必須重新啟動啟動器才能使用新的儲存位置。",
            "ja_JP": "新しい保存先を使用するには、ランチャーを再起動する必要があります。",
            "ko_KR": "새 저장 위치를 사용하려면 런처를 다시 시작해야 합니다.",
            "de_DE": "Der Launcher muss neu gestartet werden, bevor der neue Speicherort verwendet wird.",
            "fr_FR": "Le launcher doit redémarrer avant que le nouvel emplacement de stockage soit utilisé.",
            "es_ES": "El launcher debe reiniciarse antes de usar la nueva ubicación de almacenamiento.",
            "pt_BR": "O launcher precisa ser reiniciado antes que o novo local de armazenamento seja usado.",
            "ru_RU": "Лаунчер необходимо перезапустить, чтобы использовать новое расположение.",
            "tr_TR": "Yeni depolama konumunun kullanılması için başlatıcının yeniden başlatılması gerekir.",
            "pl_PL": "Launcher musi zostać ponownie uruchomiony, zanim nowa lokalizacja magazynu zostanie użyta.",
        },
        "Later": {
            "zh_CN": "稍后",
            "zh_TW": "稍後",
            "ja_JP": "後で",
            "ko_KR": "나중에",
            "de_DE": "Später",
            "fr_FR": "Plus tard",
            "es_ES": "Más tarde",
            "pt_BR": "Mais tarde",
            "ru_RU": "Позже",
            "tr_TR": "Daha sonra",
            "pl_PL": "Później",
        },
        "Restart now": {
            "zh_CN": "立即重启",
            "zh_TW": "立即重新啟動",
            "ja_JP": "今すぐ再起動",
            "ko_KR": "지금 재시작",
            "de_DE": "Jetzt neu starten",
            "fr_FR": "Redémarrer maintenant",
            "es_ES": "Reiniciar ahora",
            "pt_BR": "Reiniciar agora",
            "ru_RU": "Перезапустить сейчас",
            "tr_TR": "Şimdi yeniden başlat",
            "pl_PL": "Uruchom ponownie teraz",
        },
    },
    "ContentText": {
        "Survival": {
            "zh_CN": "生存",
            "zh_TW": "生存",
            "ja_JP": "サバイバル",
            "ko_KR": "서바이벌",
            "de_DE": "Überleben",
            "fr_FR": "Survie",
            "es_ES": "Supervivencia",
            "pt_BR": "Sobrevivência",
            "ru_RU": "Выживание",
            "tr_TR": "Hayatta Kalma",
            "pl_PL": "Przetrwanie",
        },
        "Creative": {
            "zh_CN": "创造",
            "zh_TW": "創造",
            "ja_JP": "クリエイティブ",
            "ko_KR": "크리에이티브",
            "de_DE": "Kreativ",
            "fr_FR": "Créatif",
            "es_ES": "Creativo",
            "pt_BR": "Criativo",
            "ru_RU": "Творческий",
            "tr_TR": "Yaratıcı",
            "pl_PL": "Kreatywny",
        },
        "Adventure": {
            "zh_CN": "冒险",
            "zh_TW": "冒險",
            "ja_JP": "アドベンチャー",
            "ko_KR": "어드벤처",
            "de_DE": "Abenteuer",
            "fr_FR": "Aventure",
            "es_ES": "Aventura",
            "pt_BR": "Aventura",
            "ru_RU": "Приключение",
            "tr_TR": "Macera",
            "pl_PL": "Przygodowy",
        },
        "Spectator": {
            "zh_CN": "旁观",
            "zh_TW": "旁觀",
            "ja_JP": "スペクテイター",
            "ko_KR": "관전자",
            "de_DE": "Zuschauer",
            "fr_FR": "Spectateur",
            "es_ES": "Espectador",
            "pt_BR": "Espectador",
            "ru_RU": "Наблюдатель",
            "tr_TR": "İzleyici",
            "pl_PL": "Obserwator",
        },
        "Cheats": {
            "zh_CN": "作弊",
            "zh_TW": "作弊",
            "ja_JP": "チート",
            "ko_KR": "치트",
            "de_DE": "Cheats",
            "fr_FR": "Triche",
            "es_ES": "Trucos",
            "pt_BR": "Truques",
            "ru_RU": "Читы",
            "tr_TR": "Hileler",
            "pl_PL": "Oszustwa",
        },
    },
    "InstancesList": {
        "No matching instances": {
            "zh_CN": "没有匹配的实例",
            "zh_TW": "沒有符合條件的實例",
            "ja_JP": "一致するインスタンスがありません",
            "ko_KR": "일치하는 인스턴스가 없습니다",
            "de_DE": "Keine passenden Instanzen",
            "fr_FR": "Aucune instance correspondante",
            "es_ES": "No hay instancias que coincidan",
            "pt_BR": "Nenhuma instância correspondente",
            "ru_RU": "Нет подходящих сборок",
            "tr_TR": "Eşleşen örnek yok",
            "pl_PL": "Brak pasujących instancji",
        },
        "Consider creating a new instance": {
            "zh_CN": "考虑创建一个新实例",
            "zh_TW": "考慮建立一個新實例",
            "ja_JP": "新しいインスタンスを作成してみましょう",
            "ko_KR": "새 인스턴스를 만들어 보세요",
            "de_DE": "Lege gegebenenfalls eine neue Instanz an",
            "fr_FR": "Envisagez de créer une nouvelle instance",
            "es_ES": "Considera crear una nueva instancia",
            "pt_BR": "Considere criar uma nova instância",
            "ru_RU": "Попробуйте создать новую сборку",
            "tr_TR": "Yeni bir örnek oluşturmayı düşünebilirsiniz",
            "pl_PL": "Rozważ utworzenie nowej instancji",
        },
    },
    # The four launch error dialogs, catalogued with their Chinese source
    # sentences; `ConfirmDeleteInstance` is catalogued the same way.
    "LaunchDialogs": {
        "To launch the game, you must have added at least one account": {
            "zh_CN": "要启动游戏，你必须至少已添加一个帐户",
            "zh_TW": "要啟動遊戲，你必須至少已新增一個帳戶",
            "ja_JP": "ゲームを起動するには、少なくとも1つのアカウントを追加する必要があります",
            "ko_KR": "게임을 실행하려면 계정을 하나 이상 추가해야 합니다",
            "de_DE": "Um das Spiel zu starten, musst du mindestens ein Konto hinzugefügt haben",
            "fr_FR": "Pour lancer le jeu, vous devez avoir ajouté au moins un compte",
            "es_ES": "Para lanzar el juego, debes haber añadido al menos una cuenta",
            "pt_BR": "Para iniciar o jogo, você precisa ter adicionado pelo menos uma conta",
            "ru_RU": "Чтобы запустить игру, нужно добавить хотя бы один аккаунт",
            "tr_TR": "Oyunu başlatmak için en az bir hesap eklemiş olmalısın",
            "pl_PL": "Aby uruchomić grę, musisz dodać co najmniej jedno konto",
        },
        "To launch the game, you must have added at least one Microsoft account": {
            "zh_CN": "要启动游戏，你必须至少已添加一个微软账户",
            "zh_TW": "要啟動遊戲，你必須至少已新增一個 Microsoft 帳戶",
            "ja_JP": "ゲームを起動するには、少なくとも1つの Microsoft アカウントを追加する必要があります",
            "ko_KR": "게임을 실행하려면 Microsoft 계정을 하나 이상 추가해야 합니다",
            "de_DE": "Um das Spiel zu starten, musst du mindestens ein Microsoft-Konto hinzugefügt haben",
            "fr_FR": "Pour lancer le jeu, vous devez avoir ajouté au moins un compte Microsoft",
            "es_ES": "Para lanzar el juego, debes haber añadido al menos una cuenta de Microsoft",
            "pt_BR": "Para iniciar o jogo, você precisa ter adicionado pelo menos uma conta da Microsoft",
            "ru_RU": "Чтобы запустить игру, нужно добавить хотя бы один аккаунт Microsoft",
            "tr_TR": "Oyunu başlatmak için en az bir Microsoft hesabı eklemiş olmalısın",
            "pl_PL": "Aby uruchomić grę, musisz dodać co najmniej jedno konto Microsoft",
        },
        "Failed to refresh the account. Check your network connection and try again, or log in to the account again": {
            "zh_CN": "刷新账户失败，请检查网络连接后重试，或重新登录该账户",
            "zh_TW": "重新整理帳戶失敗，請檢查網路連線後重試，或重新登入該帳戶",
            "ja_JP": "アカウントの更新に失敗しました。ネットワーク接続を確認して再試行するか、もう一度ログインしてください",
            "ko_KR": "계정을 새로 고치지 못했습니다. 네트워크 연결을 확인한 후 다시 시도하거나 계정에 다시 로그인하세요",
            "de_DE": "Das Konto konnte nicht aktualisiert werden. Prüfe die Netzwerkverbindung und versuche es erneut, oder melde dich erneut am Konto an",
            "fr_FR": "Échec de l'actualisation du compte. Vérifie ta connexion réseau et réessaie, ou reconnecte-toi au compte",
            "es_ES": "No se pudo refrescar la cuenta. Comprueba tu conexión de red e inténtalo de nuevo, o vuelve a iniciar sesión en la cuenta",
            "pt_BR": "Falha ao atualizar a conta. Verifique sua conexão com a internet e tente novamente, ou entre na conta novamente",
            "ru_RU": "Не удалось обновить аккаунт. Проверьте подключение к сети и повторите попытку или войдите в аккаунт заново",
            "tr_TR": "Hesap yenilenemedi. Ağ bağlantını kontrol edip yeniden dene ya da hesaba yeniden giriş yap",
            "pl_PL": "Nie udało się odświeżyć konta. Sprawdź połączenie sieciowe i spróbuj ponownie albo zaloguj się ponownie",
        },
        "Could not find a suitable Java runtime": {
            "zh_CN": "无法找到最合适的 Java 运行环境",
            "zh_TW": "無法找到最合適的 Java 執行環境",
            "ja_JP": "適切な Java ランタイムが見つかりませんでした",
            "ko_KR": "적절한 Java 런타임을 찾을 수 없습니다",
            "de_DE": "Es konnte keine passende Java-Laufzeitumgebung gefunden werden",
            "fr_FR": "Aucune version de Java adaptée n'a été trouvée",
            "es_ES": "No se pudo encontrar un entorno de Java adecuado",
            "pt_BR": "Não foi encontrado um ambiente Java adequado",
            "ru_RU": "Подходящая среда Java не найдена",
            "tr_TR": "Uygun bir Java çalışma zamanı bulunamadı",
            "pl_PL": "Nie znaleziono odpowiedniego środowiska uruchomieniowego Javy",
        },
        "You can set one manually in the instance settings": {
            "zh_CN": "你可以在实例设置中手动指定一个",
            "zh_TW": "你可以在實例設定中手動指定一個",
            "ja_JP": "インスタンス設定でを手動で指定できます",
            "ko_KR": "인스턴스 설정에서 직접 지정할 수 있습니다",
            "de_DE": "Du kannst sie in den Instanzeinstellungen manuell festlegen",
            "fr_FR": "Tu peux en définir une manuellement dans les paramètres de l'instance",
            "es_ES": "Puedes definir una manualmente en los ajustes de la instancia",
            "pt_BR": "Você pode definir uma manualmente nas configurações da instância",
            "ru_RU": "Её можно указать вручную в настройках сборки",
            "tr_TR": "Örnek ayarlarından elle bir tane belirtebilirsin",
            "pl_PL": "Możesz ją wskazać ręcznie w ustawieniach instancji",
        },
        "Cancel launch": {
            "zh_CN": "取消启动",
            "zh_TW": "取消啟動",
            "ja_JP": "起動をキャンセル",
            "ko_KR": "실행 취소",
            "de_DE": "Start abbrechen",
            "fr_FR": "Annuler le lancement",
            "es_ES": "Cancelar el inicio",
            "pt_BR": "Cancelar início",
            "ru_RU": "Отменить запуск",
            "tr_TR": "Başlatmayı iptal et",
            "pl_PL": "Anuluj uruchamianie",
        },
    },
    # The save and the running task, whose source sentences are Chinese and
    # which had never been internationalized.
    "ConfirmDeleteSave": {
        "是否确认删除存档「{}」？": {
            "en_US": "Delete the save “{}”?",
            "zh_TW": "是否確認刪除存檔「{}」？",
            "ja_JP": "セーブ「{}」を削除してもよろしいですか？",
            "ko_KR": "세ave 「{}」을(를) 삭제할까요?".replace("セave ", "저장 "),
            "de_DE": "Speicherstand „{}“ löschen?",
            "fr_FR": "Supprimer la sauvegarde « {} » ?",
            "es_ES": "¿Eliminar la partida «{}»?",
            "pt_BR": "Excluir o salvamento “{}”?",
            "ru_RU": "Удалить сохранение «{}»?",
            "tr_TR": "“{}” kaydı silinsin mi?",
            "pl_PL": "Usunąć zapis „{}”?",
        },
        "存档文件夹将永久删除，这是最后的反悔机会": {
            "en_US": "The save folder will be deleted permanently — this is your last chance to change your mind",
            "zh_TW": "存檔資料夾將永久刪除，這是最後的反悔機會",
            "ja_JP": "セーブフォルダは完全に削除されます。これは最後の一度です",
            "ko_KR": "세이브 폴더가 영구적으로 삭제됩니다. 마지막으로 취소할 수 있는 기회입니다",
            "de_DE": "Der Speicherordner wird endgültig gelöscht — dies ist deine letzte Chance, es dir anders zu überlegen",
            "fr_FR": "Le dossier de sauvegarde sera définitivement supprimé — c'est ta dernière chance de changer d'avis",
            "es_ES": "La carpeta de la partida se eliminará de forma permanente: esta es tu última oportunidad de arrepentirte",
            "pt_BR": "A pasta do salvamento será excluída permanentemente — esta é sua última chance de mudar de ideia",
            "ru_RU": "Папка сохранения будет удалена безвозвратно — это последняя возможность передумать",
            "tr_TR": "Kayıt klasörü kalıcı olarak silinecek — fikrini değiştirmek için son şansın",
            "pl_PL": "Folder zapisu zostanie trwale usunięty — to ostatnia szansa na zmianę zdania",
        },
        "稍等一下...": {
            "en_US": "Wait a moment…",
            "zh_TW": "稍等一下...",
            "ja_JP": "少しだけ待って…",
            "ko_KR": "잠시만 기다려요…",
            "de_DE": "Warte kurz…",
            "fr_FR": "Attends un instant…",
            "es_ES": "Espera un momento…",
            "pt_BR": "Espere um momento…",
            "ru_RU": "Подождите…",
            "tr_TR": "Biraz bekle…",
            "pl_PL": "Chwilka…",
        },
        "确认删除": {
            "en_US": "Confirm delete",
            "zh_TW": "確認刪除",
            "ja_JP": "削除する",
            "ko_KR": "삭제 확인",
            "de_DE": "Löschen bestätigen",
            "fr_FR": "Confirmer la suppression",
            "es_ES": "Confirmar el borrado",
            "pt_BR": "Confirmar exclusão",
            "ru_RU": "Подтвердить удаление",
            "tr_TR": "Silmeyi onayla",
            "pl_PL": "Potwierdź usunięcie",
        },
    },
    "ConfirmQuitApp": {
        "正在进行的任务将被中止，你确定要退出吗？": {
            "en_US": "The task in progress will be aborted. Are you sure you want to quit?",
            "zh_TW": "正在進行的任務將被中止，你確定要退出嗎？",
            "ja_JP": "実行中のタスクは中止されます。終了してもよろしいですか？",
            "ko_KR": "진행 중인 작업이 중단됩니다. 종료하시겠습니까?",
            "de_DE": "Die laufende Aufgabe wird abgebrochen. Willst du das wirklich beenden?",
            "fr_FR": "La tâche en cours sera interrompue. Es-tu sûr de vouloir quitter ?",
            "es_ES": "La tarea en curso se interrumpirá. ¿Seguro que quieres salir?",
            "pt_BR": "A tarefa em andamento será interrompida. Tem certeza de que deseja sair?",
            "ru_RU": "Текущая задача будет прервана. Вы уверены, что хотите выйти?",
            "tr_TR": "Devam eden görev iptal edilecek. Çıkmak istediğinizden emin misiniz?",
            "pl_PL": "Trwające zadanie zostanie przerwane. Czy na pewno chcesz wyjść?",
        },
        "最后的反悔机会": {
            "en_US": "This is your last chance to change your mind",
            "zh_TW": "最後的反悔機會",
            "ja_JP": "これは最後の一度です",
            "ko_KR": "마지막으로 취소할 수 있는 기회입니다",
            "de_DE": "Das ist deine letzte Chance, es dir anders zu überlegen",
            "fr_FR": "C'est ta dernière chance de changer d'avis",
            "es_ES": "Esta es tu última oportunidad de arrepentirte",
            "pt_BR": "Esta é sua última chance de mudar de ideia",
            "ru_RU": "Это последняя возможность передумать",
            "tr_TR": "Fikrini değiştirmek için son şansın",
            "pl_PL": "To ostatnia szansa na zmianę zdania",
        },
        "稍等一下...": {
            "en_US": "Wait a moment…",
            "zh_TW": "稍等一下...",
            "ja_JP": "少しだけ待って…",
            "ko_KR": "잠시만 기다려요…",
            "de_DE": "Warte kurz…",
            "fr_FR": "Attends un instant…",
            "es_ES": "Espera un momento…",
            "pt_BR": "Espere um momento…",
            "ru_RU": "Подождите…",
            "tr_TR": "Biraz bekle…",
            "pl_PL": "Chwilka…",
        },
        "让我出去！": {
            "en_US": "Let me out!",
            "zh_TW": "讓我出去！",
            "ja_JP": "閉じさせて！",
            "ko_KR": "나가게 해 주세요!",
            "de_DE": "Lass mich raus!",
            "fr_FR": "Laissez-moi sortir !",
            "es_ES": "¡Déjame salir!",
            "pt_BR": "Me deixa sair!",
            "ru_RU": "Выпустите меня!",
            "tr_TR": "Beni çıkar!",
            "pl_PL": "Wypuśćcie mnie!",
        },
    },
}

# The empty state each remote list draws. The translations were already in every
# catalogue, under `RemoteListView`.
TRY_AGAIN = "Try adjusting keywords or filters and search again"


def unescape(text: str) -> str:
    return text.replace('\\"', '"').replace("\\\\", "\\")


def escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def parse_po(path: Path) -> tuple[list[tuple[str | None, str, str]], str]:
    """Reads a catalogue into an ordered list of `(context, msgid, msgstr)` and
    the header that preceded them.

    The header comes back as the raw text it was written as. It is a `msgid ""`
    whose `msgstr` is a multi-line blob of `\n`-terminated metadata, and
    round-tripping it through the parser would re-escape the escapes — so it is
    sliced out and handed back untouched.
    """
    text = path.read_text(encoding="utf-8")
    entries: list[tuple[str | None, str, str]] = []
    ctx = msgid = msgstr = None
    field = None
    for raw in text.split("\n"):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        # The parentheses matter: `x := a and b` binds `x` to the whole `and`,
        # so an unwrapped walrus here would make `match` the `field` string on
        # the iterations where `field` is falsy.
        if (found := re.fullmatch(r'msgctxt\s+"(.*)"', line)) is not None:
            if msgid:
                entries.append((ctx, msgid, msgstr or ""))
                ctx = msgid = msgstr = None
            ctx, field = unescape(found.group(1)), "ctx"
        elif (found := re.fullmatch(r'msgid\s+"(.*)"', line)) is not None:
            if field == "str" and msgid:
                entries.append((ctx, msgid, msgstr or ""))
                ctx = msgid = msgstr = None
            msgid, field = unescape(found.group(1)), "id"
        elif (found := re.fullmatch(r'msgstr\s+"(.*)"', line)) is not None:
            msgstr, field = unescape(found.group(1)), "str"
        elif field and (found := re.fullmatch(r'"(.*)"', line)) is not None:
            if field == "ctx":
                ctx += unescape(found.group(1))
            elif field == "id":
                msgid += unescape(found.group(1))
            else:
                msgstr += unescape(found.group(1))
    if msgid:
        entries.append((ctx, msgid, msgstr or ""))
    return entries, header_block(text)


def header_block(text: str) -> str:
    """The comment lines and the `msgid ""` entry, verbatim and without the blank
    line that follows it."""
    lines = text.split("\n")
    start = next(
        (index for index, line in enumerate(lines) if line.startswith("msgid \"\"")), None
    )
    if start is None:
        return ""
    # Back up over the `#` comments that introduce the catalogue.
    while start > 0 and lines[start - 1].startswith("#"):
        start -= 1
    end = start
    while end < len(lines) and not (
        lines[end].startswith("msgctxt ") or lines[end].startswith("#: ")
    ):
        end += 1
    return "\n".join(lines[start:end]).rstrip()


def render(header: str, entries: list[tuple[str | None, str, str]]) -> str:
    out = [header, ""]
    for ctx, msgid, msgstr in entries:
        out.append("#: ui")
        if ctx is not None:
            out.append(f'msgctxt "{escape(ctx)}"')
        out.append(f'msgid "{escape(msgid)}"')
        out.append(f'msgstr "{escape(msgstr)}"')
        out.append("")
    return "\n".join(out)


def main() -> int:
    pot_path = Path(sys.argv[1] if len(sys.argv) > 1 else "messages.pot")
    pot, _ = parse_po(pot_path)
    wanted = {(ctx, msgid) for ctx, msgid, _ in pot if msgid}

    untranslated: list[tuple[str, str, str]] = []
    for language in LANGUAGES:
        path = I18N / language / "LC_MESSAGES" / f"{DOMAIN}.po"
        entries, header = parse_po(path)
        by_key = {(ctx, msgid): msgstr for ctx, msgid, msgstr in entries}

        # Drop what no longer has a component, re-key what moved, add what is new.
        result: list[tuple[str | None, str, str]] = []
        for ctx, msgid, msgstr in entries:
            if (ctx, msgid) in DROP:
                continue
            if (ctx, msgid) in RECONTEXT:
                for target in RECONTEXT[(ctx, msgid)]:
                    result.append((target, msgid, msgstr))
                continue
            result.append((ctx, msgid, msgstr))

        present = {(ctx, msgid) for ctx, msgid, _ in result}
        additions: list[tuple[str | None, str, str]] = []
        for context, sentences in SAVE_TAGS.items():
            for msgid, translations in sentences.items():
                key = (context, msgid)
                if key in present:
                    continue
                if key not in wanted:
                    print(f"  skipping {key!r}: not in the .pot")
                    continue
                # The table is asked first, because an ASCII msgid *is* its own
                # English translation and so carries no `en_US` row, while a
                # Chinese one is only the source language and does carry one.
                value = translations.get(language)
                if value is None and language == "en_US":
                    # `en_US` is the fallback: the msgid is the English source.
                    value = msgid
                if value is None and language == "zh_CN" and not msgid.isascii():
                    # The Chinese msgids are the source language — those dialogs
                    # were never internationalized — so there is nothing to look
                    # up for `zh_CN` and the source is the right answer.
                    value = msgid
                if value is None:
                    untranslated.append((language, context, msgid))
                    continue
                additions.append((context, msgid, value))
        # `zh_CN` is the source language of the Chinese msgids.
        for context, msgid, _ in list(additions):
            if msgid == msgid and language == "zh_CN" and (context, msgid) not in wanted:
                pass
        present |= {(ctx, msgid) for ctx, msgid, _ in additions}

        # The remote lists' empty state, re-keyed above from `RemoteListView`.
        for panel in ("ContentModsPanel", "ContentResourcepacksPanel", "ContentPacksPanel"):
            key = (panel, TRY_AGAIN)
            if key in present or key not in wanted:
                continue
            source = by_key.get(("RemoteListView", TRY_AGAIN), TRY_AGAIN)
            additions.append((panel, TRY_AGAIN, source))
        present |= {(ctx, msgid) for ctx, msgid, _ in additions}

        path.write_text(render(header, result + additions), encoding="utf-8")
        print(f"{language}: {len(result)} + {len(additions)} = {len(result + additions)}")

    if untranslated:
        print("\nno translation in the table for:")
        for row in untranslated:
            print("   ", row)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
