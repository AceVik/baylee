extends Control

## The duel table: seat lines, the prompt, your hand as card images, and the
## buttons the current choice actually accepts.
##
## Everything shown here is decided in Rust — the hand order, whether a card is
## playable, the threat read, which answers exist. This script only draws it.
## Card art streams from the Scryfall CDN on first use, so the first minute
## needs a network connection.

## Scryfall's `small` rendition, which is what the image policy asks for.
const CARD_SIZE := Vector2(146, 204)

@onready var duel: BayleeDuel = $Duel

var _textures: Dictionary = {}   ## url -> Texture2D
var _inflight: Dictionary = {}   ## url -> true while a request is open
var _waiting: Dictionary = {}    ## url -> Array[TextureRect] awaiting that image

var _seat_label: Label
var _prompt_label: Label
var _hand_row: HBoxContainer
var _button_row: HBoxContainer


func _ready() -> void:
	_build_ui()
	# Connect before starting: Godot runs a child's _ready before its parent's,
	# so a duel with `autostart` on would deal the opening hand before this
	# line and the first choice would be lost.
	duel.board_changed.connect(_on_board_changed)
	duel.choice_offered.connect(_on_choice_offered)
	duel.duel_failed.connect(_on_duel_failed)
	duel.start_demo_duel()


func _build_ui() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)

	var bg := ColorRect.new()
	bg.color = Color(0.07, 0.10, 0.09)
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(bg)

	var column := VBoxContainer.new()
	column.set_anchors_preset(Control.PRESET_FULL_RECT)
	column.offset_left = 24
	column.offset_top = 24
	column.offset_right = -24
	column.offset_bottom = -24
	column.add_theme_constant_override("separation", 14)
	add_child(column)

	_seat_label = Label.new()
	column.add_child(_seat_label)

	_prompt_label = Label.new()
	_prompt_label.add_theme_font_size_override("font_size", 22)
	column.add_child(_prompt_label)

	_button_row = HBoxContainer.new()
	_button_row.add_theme_constant_override("separation", 10)
	column.add_child(_button_row)

	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(spacer)

	_hand_row = HBoxContainer.new()
	_hand_row.add_theme_constant_override("separation", 8)
	column.add_child(_hand_row)


func _on_board_changed() -> void:
	var lives := duel.life_totals()
	var threats := duel.threat_lines()
	var lines := PackedStringArray()
	lines.append("turn %d · %s" % [duel.turn(), duel.step_name()])
	for i in lives.size():
		var who := "you" if i == 0 else "opponent"
		lines.append("%s — %d life · %s" % [who, lives[i], threats[i]])
	_seat_label.text = "\n".join(lines)

	# The budget decides what the renderer may drop; honour it before growing
	# past a duel, not after.
	for url in duel.evicted_urls():
		_textures.erase(url)

	_refresh_hand()
	_refresh_buttons()


func _on_choice_offered(_headline: String) -> void:
	_refresh_buttons()


func _on_duel_failed(message: String) -> void:
	push_error("duel failed: %s" % message)
	_prompt_label.text = message


func _refresh_hand() -> void:
	for child in _hand_row.get_children():
		_hand_row.remove_child(child)
		child.queue_free()
	for card in duel.hand_cards():
		_hand_row.add_child(_make_card(card))


func _make_card(card: Dictionary) -> Control:
	var slot := Control.new()
	slot.custom_minimum_size = CARD_SIZE

	var rect := TextureRect.new()
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_SCALE
	# Unplayable cards are dimmed rather than hidden: knowing what you are
	# holding and cannot cast is most of what a hand tells you.
	rect.modulate = Color.WHITE if card["playable"] else Color(0.5, 0.5, 0.55)
	slot.add_child(rect)

	# Shown until the art arrives, and kept forever for a printing whose id
	# Scryfall would 404 on.
	var fallback := Label.new()
	fallback.text = "%s\n(%d)" % [card["name"], card["mana_value"]]
	fallback.set_anchors_preset(Control.PRESET_FULL_RECT)
	fallback.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fallback.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	fallback.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	slot.add_child(fallback)

	slot.tooltip_text = card["name"]
	_want_texture(card["url"], rect, fallback)
	return slot


func _want_texture(url: String, rect: TextureRect, fallback: Label) -> void:
	if url.is_empty():
		return
	if _textures.has(url):
		rect.texture = _textures[url]
		fallback.hide()
		return

	if not _waiting.has(url):
		_waiting[url] = []
	_waiting[url].append([rect, fallback])

	# One request per URL, however many cards are waiting on it.
	if _inflight.has(url):
		return
	_inflight[url] = true

	var request := HTTPRequest.new()
	add_child(request)
	request.request_completed.connect(
		func(result: int, code: int, _headers: PackedStringArray, body: PackedByteArray) -> void:
			request.queue_free()
			_inflight.erase(url)
			if result != HTTPRequest.RESULT_SUCCESS or code != 200:
				push_warning("card art unavailable (result %d, HTTP %d): %s" % [result, code, url])
				_waiting.erase(url)
				return
			var image := Image.new()
			if image.load_jpg_from_buffer(body) != OK:
				push_warning("card art did not decode: %s" % url)
				_waiting.erase(url)
				return
			var texture := ImageTexture.create_from_image(image)
			_textures[url] = texture
			for pair in _waiting.get(url, []):
				# The hand is rebuilt on every view, so a waiting rect may
				# already have been freed.
				if is_instance_valid(pair[0]):
					pair[0].texture = texture
					pair[1].hide()
			_waiting.erase(url)
	)
	var headers := PackedStringArray([
		"User-Agent: baylee-client-godot/0.1 (unofficial fan project)",
		"Accept: image/jpeg",
	])
	if request.request(url, headers) != OK:
		push_warning("could not start image request: %s" % url)
		_inflight.erase(url)
		request.queue_free()


func _refresh_buttons() -> void:
	for child in _button_row.get_children():
		_button_row.remove_child(child)
		child.queue_free()

	_prompt_label.text = duel.prompt_headline()

	# Branch on the stable tag, never on the headline — that is prose for a
	# human and will be reworded.
	match duel.prompt_kind():
		"mulligan":
			_add_button("Keep", func() -> void: duel.answer_mulligan(true))
			_add_button("Mulligan", func() -> void: duel.answer_mulligan(false))
		"priority":
			_add_button("Pass", func() -> void: duel.pass_priority())
		"yes_no":
			_add_button("Yes", func() -> void: duel.answer_yes_no(true))
			_add_button("No", func() -> void: duel.answer_yes_no(false))
		"game_over":
			pass
		_:
			# Every other choice needs selection UI that does not exist yet.
			pass


func _add_button(label: String, action: Callable) -> void:
	var button := Button.new()
	button.text = label
	button.pressed.connect(action)
	_button_row.add_child(button)
