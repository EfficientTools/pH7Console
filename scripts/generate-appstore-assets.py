#!/usr/bin/env python3
"""Compose and validate Apple-compliant Mac screenshots from app captures."""

from pathlib import Path
from shutil import copy2

from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter, ImageFont


ROOT = Path(__file__).resolve().parents[1]
BACKGROUND = ROOT / "app-store/assets/store-background-v4.png"
ICON = ROOT / "app-store/assets/AppIcon-1024.png"
RAW = ROOT / "app-store/raw-screenshots"
OUTPUT = ROOT / "app-store/screenshots"
FASTLANE = ROOT / "fastlane/screenshots/en-US"
SIZE = (2880, 1800)
APP_MAX_SIZE = (2560, 1450)
APP_TOP = 350

CAPTURES = [
    (
        "01-private-local-assistance.png",
        "Ask naturally. Stay in control.",
        "Private local AI turns intent into a risk-rated command plan. Nothing runs until you choose.",
        "ON-DEVICE AI",
        "01-private-command-console.png",
        (0, 65, 2880, 1685),
    ),
    (
        "02-local-error-fix.png",
        "Fix errors without breaking flow.",
        "See the failure, review a precise correction, and keep moving.",
        "LOCAL ERROR RECOVERY",
        "02-local-error-guidance.png",
        (510, 65, 2880, 1415),
    ),
    (
        "03-workspace-explorer.png",
        "Your project, fully in context.",
        "Files, Git context, a live shell, and private local AI share one focused workspace.",
        "PROJECT-AWARE",
        "03-private-workspace-explorer.png",
        (0, 65, 2880, 1685),
    ),
    (
        "04-searchable-history.png",
        "Find any command in seconds.",
        "Search encrypted local history with status and timing at a glance.",
        "ENCRYPTED ON THIS MAC",
        "04-searchable-command-history.png",
        (300, 205, 2580, 1480),
    ),
    (
        "05-privacy-settings.png",
        "Local intelligence. Zero cloud.",
        "No account. No telemetry. Your terminal context stays on this Mac.",
        "PRIVATE BY DESIGN",
        "05-private-by-design.png",
        (420, 170, 2460, 1510),
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
    crop_box: tuple[int, int, int, int],
    index: int,
) -> Path:
    background = Image.open(BACKGROUND).convert("RGB").resize(SIZE, Image.Resampling.LANCZOS)
    background = ImageEnhance.Contrast(background).enhance(1.06)
    background = ImageEnhance.Brightness(background).enhance(0.78)

    # Preserve the restrained edge motion while reserving a calm,
    # high-contrast band for store copy and a readable product window.
    overlay = Image.new("RGBA", SIZE, (2, 5, 14, 0))
    overlay_draw = ImageDraw.Draw(overlay)
    overlay_draw.rectangle((0, 0, SIZE[0], 318), fill=(2, 5, 14, 184))
    overlay_draw.rectangle((0, 318, SIZE[0], SIZE[1]), fill=(2, 5, 14, 42))
    background = Image.alpha_composite(background.convert("RGBA"), overlay)
    draw = ImageDraw.Draw(background)

    icon = Image.open(ICON).convert("RGBA").resize((70, 70), Image.Resampling.LANCZOS)
    icon_mask = rounded_mask(icon.size, 16)
    icon.putalpha(ImageChops.multiply(icon.getchannel("A"), icon_mask))
    background.alpha_composite(icon, (150, 35))

    draw.text((240, 53), "pH7Console", font=font(32), fill=(225, 255, 234, 255))

    headline_font = fit_text(draw, headline, 2360, 92)
    draw.text(
        (150, 116),
        headline,
        font=headline_font,
        fill=(250, 252, 255, 255),
    )
    draw.text((153, 245), subtitle, font=font(35), fill=(201, 211, 231, 255))

    eyebrow_font = font(20)
    eyebrow_bounds = draw.textbbox((0, 0), eyebrow, font=eyebrow_font)
    eyebrow_width = eyebrow_bounds[2] - eyebrow_bounds[0]
    eyebrow_x = 2670 - eyebrow_width
    draw.rounded_rectangle(
        (eyebrow_x - 26, 47, 2718, 97),
        radius=23,
        fill=(31, 130, 94, 218),
        outline=(81, 231, 167, 190),
        width=2,
    )
    draw.text((eyebrow_x, 61), eyebrow, font=eyebrow_font, fill=(236, 255, 247, 255))

    source = load_capture(source_name).crop(crop_box)
    scale = min(APP_MAX_SIZE[0] / source.width, APP_MAX_SIZE[1] / source.height)
    app_size = (round(source.width * scale), round(source.height * scale))
    app_position = ((SIZE[0] - app_size[0]) // 2, APP_TOP)
    source = source.resize(app_size, Image.Resampling.LANCZOS).convert("RGBA")
    source.putalpha(rounded_mask(app_size, 32))

    shadow = Image.new("RGBA", (app_size[0] + 180, app_size[1] + 130), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow)
    shadow_draw.rounded_rectangle(
        (90, 36, 90 + app_size[0], 36 + app_size[1]),
        radius=42,
        fill=(0, 0, 0, 238),
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(52))
    app_x, app_y = app_position
    background.alpha_composite(shadow, (app_x - 90, app_y - 36))

    # The edge separates the real app capture from every background variation.
    edge = Image.new("RGBA", app_size, (0, 0, 0, 0))
    edge_draw = ImageDraw.Draw(edge)
    edge_draw.rounded_rectangle(
        (1, 1, app_size[0] - 2, app_size[1] - 2),
        radius=32,
        outline=(113, 150, 174, 178),
        width=3,
    )
    background.alpha_composite(source, app_position)
    background.alpha_composite(edge, app_position)

    # The sequence marker stays quiet while making gallery order intentional.
    sequence = f"0{index}"
    draw.ellipse((2760, 120, 2824, 184), fill=(14, 20, 31, 224), outline=(99, 218, 151, 190), width=2)
    sequence_bounds = draw.textbbox((0, 0), sequence, font=font(20))
    sequence_width = sequence_bounds[2] - sequence_bounds[0]
    draw.text((2792 - sequence_width / 2, 140), sequence, font=font(20), fill=(226, 255, 235, 255))

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
