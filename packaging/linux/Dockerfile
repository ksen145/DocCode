# Окружение сборки DocCode под Linux (deb + AppImage).
#
# Ubuntu 22.04 выбрана намеренно: glibc 2.35 — собранные пакеты запускаются
# на Ubuntu 22.04+, Debian 12+, Fedora 36+, Arch и большинстве свежих
# дистрибутивах.
#
# Сборка:  powershell -File packaging\linux\docker-build.ps1
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Тулчейн + системные зависимости.
#   libssl-dev     — reqwest/native-tls (OpenSSL)
#   libx11/xi/xcursor/xrandr/xkbcommon/wayland — winit (окно)
#   libgl/vulkan   — wgpu (рендер)
#   squashfs-tools, desktop-file-utils — appimagetool
#   python3-pil    — генерация иконок, если их ещё нет в дереве
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        file \
        git \
        pkg-config \
        libssl-dev \
        libx11-dev \
        libxi-dev \
        libxcursor-dev \
        libxrandr-dev \
        libxkbcommon-dev \
        libwayland-dev \
        libgl1-mesa-dev \
        libvulkan-dev \
        squashfs-tools \
        desktop-file-utils \
        python3-pil \
    && rm -rf /var/lib/apt/lists/*

# Rust (минимальный профиль: без docs/std для других целей).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path \
    && rustc --version && cargo --version

WORKDIR /src
