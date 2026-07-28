extends SceneTree

# One-off generator for the Win95 bevel textures used by the theme.
# Run with: godot --headless --path . --script tools/gen_theme_assets.gd

const FACE := Color8(192, 192, 192)
const HIGHLIGHT := Color8(255, 255, 255)
const SHADOW := Color8(128, 128, 128)
const DARK_SHADOW := Color8(0, 0, 0)

func _init() -> void:
	_generate_bevel("res://assets/theme/bevel_raised.png", HIGHLIGHT, SHADOW)
	_generate_bevel("res://assets/theme/bevel_sunken.png", SHADOW, HIGHLIGHT)
	_generate_double_bevel("res://assets/theme/window_frame.png")
	print("theme assets generated")
	quit()

func _generate_bevel(path: String, top_left_color: Color, bottom_right_color: Color) -> void:
	var size := 8
	var img := Image.create(size, size, false, Image.FORMAT_RGBA8)
	img.fill(FACE)
	for x in range(size):
		img.set_pixel(x, 0, top_left_color)
		img.set_pixel(x, size - 1, bottom_right_color)
	for y in range(size):
		img.set_pixel(0, y, top_left_color)
		img.set_pixel(size - 1, y, bottom_right_color)
	img.save_png(path)

func _generate_double_bevel(path: String) -> void:
	var size := 12
	var img := Image.create(size, size, false, Image.FORMAT_RGBA8)
	img.fill(FACE)
	for x in range(size):
		img.set_pixel(x, 0, DARK_SHADOW)
		img.set_pixel(x, size - 1, DARK_SHADOW)
	for y in range(size):
		img.set_pixel(0, y, DARK_SHADOW)
		img.set_pixel(size - 1, y, DARK_SHADOW)
	for x in range(1, size - 1):
		img.set_pixel(x, 1, HIGHLIGHT)
		img.set_pixel(x, size - 2, SHADOW)
	for y in range(1, size - 1):
		img.set_pixel(1, y, HIGHLIGHT)
		img.set_pixel(size - 2, y, SHADOW)
	img.save_png(path)
