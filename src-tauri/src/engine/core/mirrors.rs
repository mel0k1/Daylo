// Зеркало BMCLAPI: библиотеки, ассеты и maven загрузчиков из России
// качаются заметно быстрее, чем с оригинальных хостов.
const MIRROR: &str = "https://bmclapi2.bangbang93.com";

// Прямой адрес → зеркальный. None — зеркалить нечего.
// Совпадение по полному префиксу «https://хост/»: чужой домен с нашим
// хостом внутри пути под замену не попадает.
pub fn mirror_url(url: &str) -> Option<String> {
    url.strip_prefix("https://libraries.minecraft.net/")
        .map(|r| format!("{MIRROR}/maven/{r}"))
        .or_else(|| {
            url.strip_prefix("https://resources.download.minecraft.net/")
                .map(|r| format!("{MIRROR}/assets/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://meta.fabricmc.net/")
                .map(|r| format!("{MIRROR}/fabric-meta/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://maven.fabricmc.net/")
                .map(|r| format!("{MIRROR}/maven/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://maven.minecraftforge.net/")
                .map(|r| format!("{MIRROR}/maven/{r}"))
        })
        .or_else(|| {
            // У neoforged репозиторий лежит под /releases, а у зеркала уже под /maven;
            // служебный /api зеркалом не закрывается — оставляем прямой путь
            url.strip_prefix("https://maven.neoforged.net/")
                .filter(|r| !r.starts_with("api/"))
                .map(|r| {
                    let r = r.strip_prefix("releases/").unwrap_or(r);
                    format!("{MIRROR}/maven/{r}")
                })
        })
        .or_else(|| {
            url.strip_prefix("https://files.minecraftforge.net/")
                .map(|r| format!("{MIRROR}/maven/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://piston-meta.mojang.com/")
                .map(|r| format!("{MIRROR}/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://launchermeta.mojang.com/")
                .map(|r| format!("{MIRROR}/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://piston-data.mojang.com/")
                .map(|r| format!("{MIRROR}/{r}"))
        })
        .or_else(|| {
            url.strip_prefix("https://launcher.mojang.com/")
                .map(|r| format!("{MIRROR}/{r}"))
        })
        .or_else(|| {
            // Агент authlib-injector для Ely.by зеркалится у BMCLAPI
            url.strip_prefix("https://authlib-injector.yushi.moe/")
                .map(|r| format!("{MIRROR}/mirrors/authlib-injector/{r}"))
        })
}

// Адреса одного ресурса в порядке попыток: зеркало (если включено) → оригинал.
// Зеркало бывает недоступно, поэтому прямой путь остаётся запасным всегда.
pub async fn routes(url: &str) -> Vec<String> {
    if !super::settings::load().use_mirrors {
        return vec![url.to_string()];
    }
    match mirror_url(url) {
        Some(m) => vec![m, url.to_string()],
        None => vec![url.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Таблица «адрес → зеркало»: покрывает все пути лаунчера — манифест,
    // клиентский jar, библиотеки Mojang, ассеты, meta и maven загрузчиков —
    // и адреса, которые зеркалить нельзя.
    #[test]
    fn mirror_table_matches_launcher_routes() {
        let cases: [(&str, Option<&str>); 13] = [
            (
                "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
                Some("https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json"),
            ),
            (
                "https://piston-data.mojang.com/v1/objects/abc/client.jar",
                Some("https://bmclapi2.bangbang93.com/v1/objects/abc/client.jar"),
            ),
            (
                "https://libraries.minecraft.net/com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar",
                Some("https://bmclapi2.bangbang93.com/maven/com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar"),
            ),
            (
                "https://resources.download.minecraft.net/17/17abc0",
                Some("https://bmclapi2.bangbang93.com/assets/17/17abc0"),
            ),
            (
                "https://meta.fabricmc.net/v2/versions/loader/1.20.1",
                Some("https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/1.20.1"),
            ),
            (
                "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar",
                Some("https://bmclapi2.bangbang93.com/maven/net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar"),
            ),
            (
                "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.0/forge-1.20.1-47.4.0-installer.jar",
                Some("https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/1.20.1-47.4.0/forge-1.20.1-47.4.0-installer.jar"),
            ),
            (
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.233/neoforge-21.1.233-installer.jar",
                Some("https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/21.1.233/neoforge-21.1.233-installer.jar"),
            ),
            ("https://api.modrinth.com/v2/search", None),
            ("https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge", None),
            (
                "https://authlib-injector.yushi.moe/artifact/latest.json",
                Some("https://bmclapi2.bangbang93.com/mirrors/authlib-injector/artifact/latest.json"),
            ),
            (
                "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar",
                Some(
                    "https://bmclapi2.bangbang93.com/mirrors/authlib-injector/artifact/56/authlib-injector-1.2.8.jar",
                ),
            ),
            ("https://meta.fabricmc.net.evil.example/v2/x", None),
        ];
        for (url, want) in cases {
            assert_eq!(mirror_url(url).as_deref(), want, "{url}");
        }
        // Похожие хосты и подделки в пути не зеркалятся
        for url in [
            "https://evil.example/libraries.minecraft.net/x.jar",
            "https://libraries.minecraft.net.evil.example/x.jar",
            "http://libraries.minecraft.net/x.jar",
        ] {
            assert_eq!(mirror_url(url), None, "{url}");
        }
    }
}
