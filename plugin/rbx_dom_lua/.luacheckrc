stds.roblox = {
	read_globals = {
		game = {
			other_fields = true,
		},

		-- Roblox globals
		"script",

		-- Extra functions
		"tick", "warn",
		"wait", "typeof",

		-- Types
		"CFrame",
		"Color3",
		"Enum",
		"Instance",
		"NumberRange",
		"Rect",
		"UDim", "UDim2",
		"Vector2", "Vector3",
		"Vector2int16", "Vector3int16",
	}
}

stds.testez = {
	read_globals = {
		"describe",
		"it", "itFOCUS", "itSKIP",
		"FOCUS", "SKIP", "HACK_NO_XPCALL",
		"expect",
	}
}

ignore = {
	"212", -- unused arguments
}

std = "lua51+roblox"

files["**/*.spec.lua"] = {
	std = "+testez",
}