readme:
    cargo readme --project-root bevy_assetio_zip -t ../README.tpl > README.md

publish *args:
    cd bevy_assetio_zip && cargo publish {{args}}
    cd bevy_assetio_zip_bundler && cargo publish {{args}}