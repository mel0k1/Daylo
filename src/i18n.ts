import { useMemo } from 'react'
import { create } from 'zustand'

// Язык интерфейса: RU по умолчанию, EN в словаре того же типа (ключи строго
// совпадают — tsc ловит опечатки и в словаре, и в t('key')). UI-настройка:
// localStorage daylo.lang, работает и в браузерном режиме
export type Lang = 'ru' | 'en'

const ru = {
  // Навигация
  'nav.play': 'Играть',
  'nav.packs': 'Сборки',
  'nav.mods': 'Моды',
  'nav.worlds': 'Миры',
  'nav.shots': 'Скриншоты',
  'nav.settings': 'Настройки',

  // Окно (кастомный титлбар)
  'win.minimize': 'Свернуть',
  'win.maximize': 'Развернуть',
  'win.restore': 'Восстановить размер',
  'win.close': 'Закрыть',

  // Общие действия
  'action.play': 'Играть',
  'action.stop': 'Остановить',
  'action.install': 'Установить',
  'action.remove': 'Убрать',
  'action.delete': 'Удалить',
  'action.save': 'Сохранить',
  'action.cancel': 'Отмена',
  'action.add': 'Добавить',
  'action.search': 'Найти',
  'action.refresh': 'Обновить',
  'action.update': 'Обновить',
  'action.restore': 'Восстановить',
  'action.updateAll': 'Обновить все',
  'action.checkNow': 'Проверить сейчас',
  'action.openCatalog': 'Открыть каталог',
  'action.openFolder': 'Открыть папку',
  'action.verify': 'Проверить',

  // Занятость
  'busy.preparing': 'Готовим…',
  'busy.installing': 'Ставим…',
  'busy.checking': 'Проверяем…',
  'busy.updating': 'Обновляем…',
  'busy.loading': 'Загрузка…',
  'busy.thinking': 'Думаем…',
  'busy.uploading': 'Загружаем…',
  'busy.exporting': 'Сохраняем…',
  'busy.importing': 'Импортируем…',
  'busy.pinging': 'Пингуем…',

  // Общие кусочки
  'status.saved': 'Сохранено.',
  'btn.saved': 'Сохранено',
  'btn.apply': 'Применить',
  'btn.copied': 'Скопировано',
  'btn.copyCode': 'Скопировать код',
  'suffix.rec': ' · рекоменд.',
  'suffix.test': ' · тест',
  'suffix.installed': ' · есть',
  'placeholder.version': 'Версия…',
  'placeholder.build': 'Сборка…',
  'core.offline': 'Ядро не запущено — режим браузера.',
  'units.mb': '{n} МБ',
  'units.gb': '{n} ГБ',
  'units.kb': '{n} КБ',
  'badge.modsCount': 'модов: {n}',
  'type.release': 'ванила',
  'type.snapshot': 'снапшот',
  'type.old_beta': 'beta',
  'type.old_alpha': 'alpha',
  'source.custom': 'Своя',
  'badge.modpack': 'сборка',
  'error.nick': 'ник: только латиница, цифры и подчёркивания, 1–16 символов',

  // Обновление лаунчера
  'update.title': 'Обновление',
  'update.available': 'Доступна v{v}',
  'update.installing': 'Устанавливаем…',
  'update.progress': 'Обновление: {pct}%',
  'update.applyAndRestart': 'Установить и перезапуститься',
  'update.fresh': 'Лаунчер свежей версии.',

  // Играть: библиотека
  'lib.packs': 'Сборки',
  'lib.packsAction': 'Каталог сборок →',
  'lib.packsEmpty': 'Готовых сборок нет — установите из каталога Modrinth, CurseForge или FTB.',
  'lib.own': 'Свои сборки',
  'lib.ownAction': 'Собрать свою →',
  'lib.ownEmpty': 'Своя сборка — ванила или загрузчик под выбранную версию, соберётся сама.',
  'lib.versions': 'Скачанные версии',
  'lib.installLoader': '+ Установить загрузчик',
  'lib.hideLoader': 'Скрыть установку',
  'lib.versionsEmpty': 'Ничего не скачано — выберите версию сверху и нажмите «Играть».',
  'play.selectVersion': 'Выбрать версию',
  'play.nick': 'Ник',
  'play.accountAuth': 'Аккаунт авторизован',
  'loader.hint': '{loader} для {mc}',
  'loader.none': 'Сборок {loader} под {mc} нет',
  'loader.checking': 'Сборки {loader}…',

  // Скин
  'skin.title': 'Скин',
  'skin.file': 'Скин PNG',
  'skin.toEly': 'На Ely.by',
  'skin.saved': 'Скин загружен в аккаунт.',
  'skin.note':
    'Скин применяется через CustomSkinLoader в сборках с загрузчиком; с аккаунтом Ely.by — через authlib-injector.',
  'skin.hint': 'скин не выбран',
  'skin.notLoaded': 'скин не загрузился',
  'error.notSkin': 'это не скин Minecraft',
  'error.noCanvas': 'нет canvas',
  'error.readFile': 'не удалось прочитать файл',
  'error.skinFile': 'сначала выберите файл скина',

  // Свои сборки (экран «Сборки»)
  'own.toggle': 'Собрать свою',
  'own.collapse': 'Свернуть',
  'own.note': 'Ванила или загрузчик под выбранную версию — качается и ставится сама.',
  'own.namePlaceholder': 'Название сборки',
  'own.pending': 'Ставим сборку — прогресс виден на экране «Играть».',
  'own.create': 'Создать',
  'own.defaultName': 'Моя сборка',

  // Сборки
  'packs.title': 'Сборки',
  'packs.mine': 'Мои сборки',
  'packs.catalog': 'Каталог',
  'packs.empty': 'Сборок пока нет.',
  'packs.emptyHint': 'Установи готовую из каталога — Modrinth, CurseForge или FTB.',
  'packs.installPlaceholder': 'установка сборки',
  'packs.ftbPlaceholder': 'Поиск в каталоге FTB…',
  'packs.searchOn': 'Поиск сборок на {src}…',
  'packs.cfNote':
    'CurseForge идёт через публичное зеркало; если оно молчит — добавь свой ключ API в настройках.',
  'packs.versionsTitle': 'Версии: {title}',
  'packs.noneForVersion': 'Под эту версию игры подходящих сборок нет.',
  'packs.noMcVersions': 'версии не указаны',
  'downloads.count': '{n} скачиваний',
  'packs.defaultName': 'Моя сборка',

  // ИИ-сборщик
  'ai.toggle': 'ИИ-сборка',
  'ai.note': 'Опиши сборку словами — моды подберёт нейросеть и поставит в новую сборку. Бесплатно.',
  'ai.promptPlaceholder':
    'Например: лёгкая техно-сборка с автоматизацией и хранением энергии, немного магии',
  'ai.planNote': 'План: {n} модов. Сними галочку с ненужных и собирай.',
  'ai.makePlan': 'Собрать план',
  'ai.create': 'Создать сборку ({n} модов)',
  'ai.none': 'Ничего не нашлось — попробуй переформулировать.',
  'ai.defaultName': 'ИИ-сборка',
  'ai.failedMods': 'Не поставились: {list}',

  // Моды
  'mods.title': 'Моды',
  'mods.mine': 'Мои моды',
  'mods.catalog': 'Каталог',
  'mods.check': 'Проверить обновления',
  'mods.updatesNote': 'Доступно обновлений: {n}.',
  'mods.emptyNote': 'В сборке {inst} пока нет модов.',
  'mods.emptyHint': 'Загляни в «Каталог» — Modrinth и CurseForge под рукой.',
  'mods.oldVersion': 'старая версия',
  'mods.searchMr': 'Поиск на Modrinth…',
  'mods.searchCf': 'Поиск на CurseForge…',
  'error.pickVersion': 'выберите версию игры, чтобы отфильтровать совместимые моды',

  // Аккаунт
  'acc.title': 'Аккаунт',
  'acc.logout': 'Выйти',
  'acc.offlineNote':
    'Оффлайн-режим без авторизации. Добавь оффлайн-профиль по нику или войди в аккаунт — профили сохраняются, переключаться можно в любой момент.',
  'acc.enterCode': 'Введите код на странице входа {provider}:',
  'acc.waiting': 'Ждём подтверждение…',
  'acc.loginEly': 'Войти через Ely.by',
  'acc.loginMsa': 'Войти через Microsoft',
  'acc.active': 'Активный аккаунт',
  'acc.makeActive': 'Сделать активным',
  'acc.remove': 'Убрать аккаунт',
  'acc.nickPlaceholder': 'Ник оффлайн-профиля',
  'acc.provider.msa': 'Microsoft',
  'acc.provider.ely': 'ely.by',
  'acc.mode.offline': 'оффлайн',
  'acc.codeExpired': 'время действия кода истекло',
  'acc.elyRejected': 'ely.by отклонил вход',

  // Настройки
  'settings.title': 'Настройки',
  'settings.memory': 'Память (МБ), по умолчанию 2048',
  'settings.flags': 'Дополнительные JVM-флаги, по одному в строке',
  'error.ram': 'память — число от 512 до 32768 МБ',

  // Зеркала и ключи
  'mirrors.title': 'Загрузки',
  'mirrors.bmclapi': 'Зеркало BMCLAPI для библиотек, ассетов и загрузчиков',
  'mirrors.fallback':
    'Оригинальные серверы Mojang и maven остаются запасным путём: если зеркало молчит, качаем напрямую.',
  'mirrors.cfKey': 'Ключ API CurseForge (необязательно)',
  'mirrors.cfNote':
    'Моды и сборки CurseForge идут через публичное зеркало. Свой ключ включает официальный API, если зеркало недоступно.',

  // Окно игры
  'window.title': 'Окно игры',
  'window.width': 'Ширина',
  'window.height': 'Высота',
  'window.fullscreen': 'Открывать игру на весь экран',
  'window.note':
    'Размер окна запускаемой игры, по умолчанию 1280x720. Применится к следующему запуску.',
  'error.width': 'ширина — число от 320 до 7680',
  'error.height': 'высота — число от 240 до 4320',

  // Бэкапы
  'backups.title': 'Бэкапы миров',
  'backups.on': 'Бэкапить миры перед запуском',
  'backups.keep': 'Копий на мир',
  'backups.note':
    'Перед запуском архивируются только миры, изменившиеся после свежайшей копии. Старые копии удаляются сами, keep-версии от восстановления живут неделю.',
  'error.keep': 'копий на мир — число от 1 до 30',

  // Экспорт/импорт сборок
  'packsCard.note':
    'Файл-сборка — один json: версия игры, память и список модов. Моды и сама игра скачаются с Modrinth и серверов Mojang при импорте.',
  'packsCard.export': 'Экспортировать текущую',
  'packsCard.import': 'Импорт из файла',
  'packsCard.savedPath': 'Сборка сохранена: {path}',
  'packsCard.installingFile': 'Ставим сборку {id}…',

  // Оформление
  'theme.title': 'Оформление',
  'theme.dark': 'Тьма',
  'theme.light': 'Светлая',
  'theme.plains': 'Равнина',
  'theme.nether': 'Ад',
  'theme.note': 'Тема применяется сразу и запоминается до следующего запуска.',
  'lang.title': 'Язык',

  // Миры
  'worlds.title': 'Миры',
  'worlds.backupsFolder': 'Папка бэкапов',
  'worlds.reading': 'Читаем миры…',
  'worlds.backingUp': 'Бэкап «{world}»…',
  'worlds.restoring': 'Восстановление…',
  'worlds.restoreConfirm':
    'Заменить мир «{world}» копией из бэкапа? Текущая версия сохранится рядом (keep-…).',
  'worlds.empty':
    'Миров нет. Запусти игру в этой сборке и создай мир — он появится здесь, и его можно будет сохранить в бэкап.',
  'worlds.played': 'Играл: {date}',
  'worlds.backups': 'Бэкапы',
  'worlds.makeBackup': 'Сделать бэкап',

  // Скриншоты
  'shots.reading': 'Читаем папку…',
  'shots.empty': 'Скриншотов нет. Снимки из игры (клавиша F2) появятся здесь автоматически.',
  'shots.shownLast': 'Показаны последние {n}. Остальные — в папке снимков.',

  // Серверы
  'servers.title': 'Серверы',
  'servers.offline': '{s}: нет связи',
  'servers.note': 'Добавьте адрес сервера — здесь появится его статус.',
  'servers.online': 'онлайн',
  'servers.down': 'недоступен',
  'servers.checking': 'проверяем…',
  'servers.removeFromList': 'Убрать из списка',
  'servers.fromGame': 'Из игры ({v})',
  'servers.track': 'Добавить в наблюдаемые',
  'servers.noConn': 'нет связи',
  'source.modrinth': 'Modrinth',
  'source.curseforge': 'CurseForge',
  'source.ftb': 'FTB',

  // Новости и краш
  'news.all': 'Все релизы',
  'crash.title': 'Игра упала',
  'crash.build': 'Сборка: {inst}',
  'crash.logSummary': 'Что в логе',
  'crash.ok': 'Понятно',

  // ErrorBoundary
  'eb.title': 'Интерфейс упал',
  'eb.renderError': 'Произошла ошибка отрисовки:',
  'eb.note':
    'Сборки, настройки и миры не пострадали. Перезапустите интерфейс — если ошибка повторяется на каждом старте, пришлите автору текст из этого экрана.',
  'eb.restart': 'Перезапустить интерфейс',

  // Состояние сборок/игры (сторы)
  'inst.verifyClean': 'Файлы проверены — всё в порядке',
  'inst.verifySummary': 'Файлы проверены: {parts}',
  'inst.filesFixed': 'исправлено файлов: {n}',
  'inst.modsFixed': 'модов восстановлено: {n}',
  'inst.modsMissing': 'не вернулись: {list}',
  'inst.launched': 'игра запущена',
  'inst.removeConfirm': 'Удалить сборку «{name}» вместе с мирами и модами?',
  'inst.removeVersionConfirm': 'Удалить версию {id} с диска?',
  'game.stageError': 'ошибка',
  'game.stagePreparing': 'подготовка',
  'game.exited': 'Игра завершилась',
  'game.exitedCode': 'Игра завершилась с кодом {code}',
}

export type TKey = keyof typeof ru

// EN-словарь типизирован через typeof ru: пропущенный или лишний ключ — ошибка tsc
const en: typeof ru = {
  'nav.play': 'Play',
  'nav.packs': 'Modpacks',
  'nav.mods': 'Mods',
  'nav.worlds': 'Worlds',
  'nav.shots': 'Screenshots',
  'nav.settings': 'Settings',

  // Окно (кастомный титлбар)
  'win.minimize': 'Minimize',
  'win.maximize': 'Maximize',
  'win.restore': 'Restore size',
  'win.close': 'Close',

  'action.play': 'Play',
  'action.stop': 'Stop',
  'action.install': 'Install',
  'action.remove': 'Remove',
  'action.delete': 'Delete',
  'action.save': 'Save',
  'action.cancel': 'Cancel',
  'action.add': 'Add',
  'action.search': 'Search',
  'action.refresh': 'Refresh',
  'action.update': 'Update',
  'action.restore': 'Restore',
  'action.updateAll': 'Update all',
  'action.checkNow': 'Check now',
  'action.openCatalog': 'Open catalog',
  'action.openFolder': 'Open folder',
  'action.verify': 'Verify',

  'busy.preparing': 'Preparing…',
  'busy.installing': 'Installing…',
  'busy.checking': 'Checking…',
  'busy.updating': 'Updating…',
  'busy.loading': 'Loading…',
  'busy.thinking': 'Thinking…',
  'busy.uploading': 'Uploading…',
  'busy.exporting': 'Saving…',
  'busy.importing': 'Importing…',
  'busy.pinging': 'Pinging…',

  'status.saved': 'Saved.',
  'btn.saved': 'Saved',
  'btn.apply': 'Apply',
  'btn.copied': 'Copied',
  'btn.copyCode': 'Copy code',
  'suffix.rec': ' · rec.',
  'suffix.test': ' · test',
  'suffix.installed': ' · installed',
  'placeholder.version': 'Version…',
  'placeholder.build': 'Build…',
  'core.offline': 'Core is not running — browser mode.',
  'units.mb': '{n} MB',
  'units.gb': '{n} GB',
  'units.kb': '{n} KB',
  'badge.modsCount': 'mods: {n}',
  'type.release': 'vanilla',
  'type.snapshot': 'snapshot',
  'type.old_beta': 'beta',
  'type.old_alpha': 'alpha',
  'source.custom': 'Custom',
  'badge.modpack': 'modpack',
  'error.nick': 'nickname: Latin letters, digits and underscores, 1–16 chars',

  'update.title': 'Update',
  'update.available': 'v{v} available',
  'update.installing': 'Installing…',
  'update.progress': 'Update: {pct}%',
  'update.applyAndRestart': 'Install and restart',
  'update.fresh': 'The launcher is up to date.',

  'lib.packs': 'Modpacks',
  'lib.packsAction': 'Browse modpacks →',
  'lib.packsEmpty': 'No modpacks yet — install one from the Modrinth, CurseForge or FTB catalog.',
  'lib.own': 'My builds',
  'lib.ownAction': 'Build your own →',
  'lib.ownEmpty':
    'Your own build — vanilla or a loader for the chosen version, it assembles itself.',
  'lib.versions': 'Downloaded versions',
  'lib.installLoader': '+ Install a loader',
  'lib.hideLoader': 'Hide installer',
  'lib.versionsEmpty': 'Nothing downloaded yet — pick a version above and hit “Play”.',
  'play.selectVersion': 'Choose version',
  'play.nick': 'Nickname',
  'play.accountAuth': 'Account authorized',
  'loader.hint': '{loader} for {mc}',
  'loader.none': 'No {loader} builds for {mc}',
  'loader.checking': 'Fetching {loader} builds…',

  'skin.title': 'Skin',
  'skin.file': 'Skin PNG',
  'skin.toEly': 'To Ely.by',
  'skin.saved': 'Skin uploaded to your account.',
  'skin.note':
    'The skin is applied via CustomSkinLoader in loader builds; with an Ely.by account — via authlib-injector.',
  'skin.hint': 'no skin selected',
  'skin.notLoaded': 'the skin failed to load',
  'error.notSkin': 'this is not a Minecraft skin',
  'error.noCanvas': 'no canvas',
  'error.readFile': 'failed to read the file',
  'error.skinFile': 'pick a skin file first',

  'own.toggle': 'Build your own',
  'own.collapse': 'Collapse',
  'own.note': 'Vanilla or a loader for the chosen version — downloads and installs itself.',
  'own.namePlaceholder': 'Build name',
  'own.pending': 'Installing the build — progress is shown on the “Play” screen.',
  'own.create': 'Create',
  'own.defaultName': 'My build',

  'packs.title': 'Modpacks',
  'packs.mine': 'My modpacks',
  'packs.catalog': 'Catalog',
  'packs.empty': 'No modpacks yet.',
  'packs.emptyHint': 'Install a ready one from the catalog — Modrinth, CurseForge or FTB.',
  'packs.installPlaceholder': 'installing a modpack',
  'packs.ftbPlaceholder': 'Search the FTB catalog…',
  'packs.searchOn': 'Search modpacks on {src}…',
  'packs.cfNote':
    'CurseForge goes through a public mirror; if it is silent — add your own API key in the settings.',
  'packs.versionsTitle': 'Versions: {title}',
  'packs.noneForVersion': 'No modpacks match this game version.',
  'packs.noMcVersions': 'versions not listed',
  'downloads.count': '{n} downloads',
  'packs.defaultName': 'My build',

  'ai.toggle': 'AI build',
  'ai.note': 'Describe your build in words — a neural net picks the mods and installs them into a new build. Free.',
  'ai.promptPlaceholder':
    'e.g. a light tech pack with automation and energy storage, a bit of magic',
  'ai.planNote': 'Plan: {n} mods. Uncheck the ones you don’t need and build.',
  'ai.makePlan': 'Build a plan',
  'ai.create': 'Create build ({n} mods)',
  'ai.none': 'Nothing found — try rephrasing.',
  'ai.defaultName': 'AI build',
  'ai.failedMods': 'Failed to install: {list}',

  'mods.title': 'Mods',
  'mods.mine': 'My mods',
  'mods.catalog': 'Catalog',
  'mods.check': 'Check for updates',
  'mods.updatesNote': '{n} updates available.',
  'mods.emptyNote': '{inst} has no mods yet.',
  'mods.emptyHint': 'Check the “Catalog” — Modrinth and CurseForge are at hand.',
  'mods.oldVersion': 'old version',
  'mods.searchMr': 'Search Modrinth…',
  'mods.searchCf': 'Search CurseForge…',
  'error.pickVersion': 'choose a game version to filter compatible mods',

  'acc.title': 'Account',
  'acc.logout': 'Log out',
  'acc.offlineNote':
    'Offline mode without authorization. Add an offline profile by nickname or sign in — profiles are saved, switch anytime.',
  'acc.enterCode': 'Enter the code on the {provider} sign-in page:',
  'acc.waiting': 'Waiting for confirmation…',
  'acc.loginEly': 'Sign in with Ely.by',
  'acc.loginMsa': 'Sign in with Microsoft',
  'acc.active': 'Active account',
  'acc.makeActive': 'Make active',
  'acc.remove': 'Remove account',
  'acc.nickPlaceholder': 'Offline profile nickname',
  'acc.provider.msa': 'Microsoft',
  'acc.provider.ely': 'ely.by',
  'acc.mode.offline': 'offline',
  'acc.codeExpired': 'the code has expired',
  'acc.elyRejected': 'ely.by rejected the sign-in',

  'settings.title': 'Settings',
  'settings.memory': 'Memory (MB), 2048 by default',
  'settings.flags': 'Extra JVM flags, one per line',
  'error.ram': 'memory — a number from 512 to 32768 MB',

  'mirrors.title': 'Downloads',
  'mirrors.bmclapi': 'BMCLAPI mirror for libraries, assets and loaders',
  'mirrors.fallback':
    'The original Mojang and maven servers remain a fallback: if the mirror is silent, we download directly.',
  'mirrors.cfKey': 'CurseForge API key (optional)',
  'mirrors.cfNote':
    'CurseForge mods and modpacks go through a public mirror. Your own key enables the official API if the mirror is unavailable.',

  'window.title': 'Game window',
  'window.width': 'Width',
  'window.height': 'Height',
  'window.fullscreen': 'Open the game fullscreen',
  'window.note': 'Window size of the launched game, 1280x720 by default. Applies to the next launch.',
  'error.width': 'width — a number from 320 to 7680',
  'error.height': 'height — a number from 240 to 4320',

  'backups.title': 'World backups',
  'backups.on': 'Back up worlds before launch',
  'backups.keep': 'Copies per world',
  'backups.note':
    'Only worlds changed since the freshest copy are archived before launch. Old copies are removed automatically; keep-versions from a restore live for a week.',
  'error.keep': 'copies per world — a number from 1 to 30',

  'packsCard.note':
    'A build file is a single json: game version, memory and a mod list. The mods and the game itself are downloaded from Modrinth and Mojang servers on import.',
  'packsCard.export': 'Export current',
  'packsCard.import': 'Import from file',
  'packsCard.savedPath': 'Build saved: {path}',
  'packsCard.installingFile': 'Installing build {id}…',

  'theme.title': 'Appearance',
  'theme.dark': 'Dark',
  'theme.light': 'Light',
  'theme.plains': 'Plains',
  'theme.nether': 'Nether',
  'theme.note': 'The theme applies instantly and is remembered until the next launch.',
  'lang.title': 'Language',

  'worlds.title': 'Worlds',
  'worlds.backupsFolder': 'Backups folder',
  'worlds.reading': 'Reading worlds…',
  'worlds.backingUp': 'Backing up “{world}”…',
  'worlds.restoring': 'Restoring…',
  'worlds.restoreConfirm':
    'Replace the world “{world}” with the backup copy? The current version will be kept nearby (keep-…).',
  'worlds.empty':
    'No worlds yet. Run the game in this build and create a world — it will appear here and can be backed up.',
  'worlds.played': 'Last played: {date}',
  'worlds.backups': 'Backups',
  'worlds.makeBackup': 'Make a backup',

  'shots.reading': 'Reading folder…',
  'shots.empty': 'No screenshots yet. In-game shots (F2 key) will appear here automatically.',
  'shots.shownLast': 'Showing the last {n}. The rest are in the screenshots folder.',

  'servers.title': 'Servers',
  'servers.offline': '{s}: no connection',
  'servers.note': 'Add a server address — its status will appear here.',
  'servers.online': 'online',
  'servers.down': 'unreachable',
  'servers.checking': 'checking…',
  'servers.removeFromList': 'Remove from list',
  'servers.fromGame': 'From the game ({v})',
  'servers.track': 'Add to watched',
  'servers.noConn': 'no connection',
  'source.modrinth': 'Modrinth',
  'source.curseforge': 'CurseForge',
  'source.ftb': 'FTB',

  'news.all': 'All releases',
  'crash.title': 'Game crashed',
  'crash.build': 'Build: {inst}',
  'crash.logSummary': 'What’s in the log',
  'crash.ok': 'Got it',

  'eb.title': 'UI crashed',
  'eb.renderError': 'A rendering error occurred:',
  'eb.note':
    'Builds, settings and worlds are safe. Restart the UI — if the error happens on every start, send the author the text from this screen.',
  'eb.restart': 'Restart the UI',

  'inst.verifyClean': 'Files checked — all in order',
  'inst.verifySummary': 'Files checked: {parts}',
  'inst.filesFixed': 'files fixed: {n}',
  'inst.modsFixed': 'mods restored: {n}',
  'inst.modsMissing': 'did not come back: {list}',
  'inst.launched': 'game launched',
  'inst.removeConfirm': 'Delete the build “{name}” along with its worlds and mods?',
  'inst.removeVersionConfirm': 'Delete version {id} from disk?',
  'game.stageError': 'error',
  'game.stagePreparing': 'preparing',
  'game.exited': 'The game has exited',
  'game.exitedCode': 'The game exited with code {code}',
}

const DICT: Record<Lang, typeof ru> = { ru, en }

const KEY = 'daylo.lang'

function load(): Lang {
  try {
    const l = localStorage.getItem(KEY)
    return l === 'en' ? 'en' : 'ru'
  } catch {
    return 'ru'
  }
}

export const useLang = create<{ lang: Lang; set: (l: Lang) => void }>((set) => ({
  lang: load(),
  set: (lang) => {
    try {
      localStorage.setItem(KEY, lang)
    } catch {
      /* приватный режим — язык просто не запомнится */
    }
    set({ lang })
  },
}))

function fmt(s: string, vars?: Record<string, string | number>): string {
  if (!vars) return s
  let out = s
  for (const [k, v] of Object.entries(vars)) out = out.split(`{${k}}`).join(String(v))
  return out
}

// t для мест вне React (сторы, обработчики): читает текущий язык на вызове
export function t(key: TKey, vars?: Record<string, string | number>): string {
  return fmt(DICT[useLang.getState().lang][key], vars)
}

// Хук для компонентов: подписка на смену языка и стабильный переводчик
export function useT(): (key: TKey, vars?: Record<string, string | number>) => string {
  const lang = useLang((s) => s.lang)
  return useMemo(
    () => (key: TKey, vars?: Record<string, string | number>) => fmt(DICT[lang][key], vars),
    [lang],
  )
}

// Локаль для дат/чисел: ru-RU или en-US
export function locale(): string {
  return useLang.getState().lang === 'en' ? 'en-US' : 'ru-RU'
}
