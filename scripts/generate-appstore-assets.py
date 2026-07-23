#!/usr/bin/env python3
"""Compose and validate Apple-compliant Mac screenshots from app captures."""

from pathlib import Path
from shutil import copy2

from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter, ImageFont


ROOT = Path(__file__).resolve().parents[1]
BACKGROUND = ROOT / "app-store/assets/store-background-v2.png"
ICON = ROOT / "app-store/assets/AppIcon-1024.png"
RAW = ROOT / "app-store/raw-screenshots"
OUTPUT = ROOT / "app-store/screenshots"
FASTLANE = ROOT / "fastlane/screenshots/en-US"
SIZE = (2880, 1800)
APP_SIZE = (2304, 1440)
APP_POSITION = (288, 345)

CAPTURES = [
    (
        "01-private-local-assistance.png",
        "YOUR SHELL. NOW WITH PRIVATE LOCAL AI.",
        "Review a safe command plan, then insert it—nothing runs without you.",
        "PRIVATE • LOCAL • IN CONTROL",
        "01-private-command-console.png",
    ),
    (
        "02-local-error-fix.png",
        "FIX THE ERROR. KEEP YOUR FLOW.",
        "Private local guidance explains the next command while you stay in control.",
        "LOCAL ERROR GUIDANCE",
        "02-local-error-guidance.png",
    ),
    (
        "03-workspace-explorer.png",
        "YOUR PROJECT. YOUR SHELL. ONE WORKSPACE.",
        "Browse the folder you choose while a real PTY stays live.",
        "WORKSPACE AWARE",
        "03-private-workspace-explorer.png",
    ),
    (
        "04-searchable-history.png",
        "YOUR COMMAND HISTORY. FAST, PRIVATE, SEARCHABLE.",
        "Search encrypted local records with exit status and timing—never command output.",
        "ENCRYPTED LOCAL RECALL",
        "04-searchable-command-history.png",
    ),
    (
        "05-privacy-settings.png",
        "YOUR WORK STAYS ON YOUR MAC.",
        "No account, no telemetry, no remote AI—and encrypted command history.",
        "PRIVATE BY DESIGN",
        "05-private-by-design.png",
    ),
]


def font(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", size=size)


def fit_text(
    draw: ImageDraw.ImageDraw,
    text: str,
    max_width: int,
    start_size: int,
    minimum_size: int = 52,
) -> ImageFont.FreeTypeFont:
    size = start_size
    while size > minimum_size:
        candidate = font(size)
        bounds = draw.textbbox((0, 0), text, font=candidate)
        if bounds[2] - bounds[0] <= max_width:
            return candidate
        size -= 2
    return font(size)


def rounded_mask(size: tuple[int, int], radius: int) -> Image.Image:
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, size[0] - 1, size[1] - 1),
        radius=radius,
        fill=255,
    )
    return mask


def load_capture(source_name: str) -> Image.Image:
    path = RAW / source_name
    source = Image.open(path)
    if source.size != SIZE:
        raise ValueError(f"{path} is {source.size}; expected a 2880x1800 Retina capture")
    if source.format != "PNG":
        raise ValueError(f"{path} is {source.format}; App Store captures must be PNG")
    return source.convert("RGB")


def compose(
    source_name: str,
    headline: str,
    subtitle: str,
    eyebrow: str,
    output_name: str,
    index: int,
) -> Path:
    background = Image.open(BACKGROUND).convert("RGB").resize(SIZE, Image.Resampling.LANCZOS)
    background = ImageEnhance.Contrast(background).enhance(1.08)
    background = ImageEnhance.Brightness(background).enhance(0.84)

    # Retain the generated green/violet motion while reserving a calm,
    # high-contrast band for store copy and a readable product window.
    overlay = Image.new("RGBA", SIZE, (2, 5, 14, 0))
    overlay_draw = ImageDraw.Draw(overlay)
    overlay_draw.rectangle((0, 0, SIZE[0], 338), fill=(2, 5, 14, 172))
    overlay_draw.rectangle((0, 338, SIZE[0], SIZE[1]), fill=(2, 5, 14, 34))
    background = Image.alpha_composite(background.convert("RGBA"), overlay)
    draw = ImageDraw.Draw(background)

    icon = Image.open(ICON).convert("RGBA").resize((76, 76), Image.Resampling.LANCZOS)
    icon_mask = rounded_mask(icon.size, 17)
    icon.putalpha(ImageChops.multiply(icon.getchannel("A"), icon_mask))
    background.alpha_composite(icon, (150, 38))

    draw.text((248, 56), "pH7Console", font=font(34), fill=(225, 255, 234, 255))
    draw.rounded_rectangle((468, 51, 698, 94), radius=21, fill=(35, 48, 69, 210))
    draw.text((499, 59), "FOR macOS", font=font(22), fill=(184, 196, 218, 255))

    headline_font = fit_text(draw, headline, 2360, 86)
    draw.text(
        (150, 123),
        headline,
        font=headline_font,
        fill=(250, 252, 255, 255),
        stroke_width=1,
        stroke_fill=(250, 252, 255, 90),
    )
    draw.text((153, 244), subtitle, font=font(36), fill=(201, 211, 231, 255))

    eyebrow_font = font(20)
    eyebrow_bounds = draw.textbbox((0, 0), eyebrow, font=eyebrow_font)
    eyebrow_width = eyebrow_bounds[2] - eyebrow_bounds[0]
    eyebrow_x = 2580 - eyebrow_width
    draw.rounded_rectangle(
        (eyebrow_x - 24, 53, 2718, 99),
        radius=23,
        fill=(103, 80, 225, 212),
        outline=(146, 126, 255, 175),
        width=2,
    )
    draw.text((eyebrow_x, 63), eyebrow, font=eyebrow_font, fill=(255, 255, 255, 255))

    source = load_capture(source_name).resize(APP_SIZE, Image.Resampling.LANCZOS).convert("RGBA")
    source.putalpha(rounded_mask(APP_SIZE, 30))

    shadow = Image.new("RGBA", (APP_SIZE[0] + 160, APP_SIZE[1] + 120), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow)
    shadow_draw.rounded_rectangle(
        (80, 34, 80 + APP_SIZE[0], 34 + APP_SIZE[1]),
        radius=38,
        fill=(0, 0, 0, 232),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(46))
    app_x, app_y = APP_POSITION
    background.alpha_composite(shadow, (app_x - 80, app_y - 34))

    # The edge separates the real app capture from every background variation.
    edge = Image.new("RGBA", APP_SIZE, (0, 0, 0, 0))
    edge_draw = ImageDraw.Draw(edge)
    edge_draw.rounded_rectangle(
        (1, 1, APP_SIZE[0] - 2, APP_SIZE[1] - 2),
        radius=30,
        outline=(129, 147, 185, 165),
        width=3,
    )
    background.alpha_composite(source, APP_POSITION)
    background.alpha_composite(edge, APP_POSITION)

    # A restrained sequence marker makes ordering obvious without competing
    # with the feature-specific badge.
    sequence = f"0{index}"
    draw.ellipse((2750, 45, 2822, 117), fill=(14, 20, 31, 224), outline=(99, 218, 151, 190), width=2)
    sequence_bounds = draw.textbbox((0, 0), sequence, font=font(22))
    sequence_width = sequence_bounds[2] - sequence_bounds[0]
    draw.text((2786 - sequence_width / 2, 67), sequence, font=font(22), fill=(226, 255, 235, 255))

    OUTPUT.mkdir(parents=True, exist_ok=True)
    output_path = OUTPUT / output_name
    background.convert("RGB").save(output_path, format="PNG", optimize=True)
    return output_path


def validate_and_sync(paths: list[Path]) -> None:
    expected_names = {path.name for path in paths}
    for directory in (OUTPUT, FASTLANE):
        directory.mkdir(parents=True, exist_ok=True)
        for stale_path in directory.glob("*.png"):
            if stale_path.name not in expected_names:
                stale_path.unlink()

    FASTLANE.mkdir(parents=True, exist_ok=True)
    for path in paths:
        with Image.open(path) as image:
            if image.size != SIZE or image.mode != "RGB" or image.format != "PNG":
                raise ValueError(
                    f"{path} failed App Store validation: "
                    f"size={image.size}, mode={image.mode}, format={image.format}"
                )
        copy2(path, FASTLANE / path.name)


if __name__ == "__main__":
    generated = [
        compose(*capture, position)
        for position, capture in enumerate(CAPTURES, start=1)
    ]
    validate_and_sync(generated)
    print(f"Generated and validated {len(generated)} 2880x1800 screenshots in {OUTPUT}")
    print(f"Synced en-US screenshots to {FASTLANE}")
