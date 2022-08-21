local strict = require(script.Parent.strict)

local Assets = {
	Sprites = {},
	Slices = {
		RoundedBackground = {
			Image = "rbxassetid://5981360418",
			Center = Rect.new(10, 10, 10, 10),
			Scale = 0.5,
		},

		RoundedBorder = {
			Image = "rbxassetid://5981360137",
			Center = Rect.new(10, 10, 10, 10),
			Scale = 0.5,
		},
	},
	Images = {
		Logo = "rbxassetid://5990772764",
		PluginButton = "rbxassetid://3405341609",
		PluginButtonConnected = "rbxassetid://9529783993",
		PluginButtonWarning = "rbxassetid://9529784530",
		Icons = {
			Close = "rbxassetid://6012985953",
			Back = "rbxassetid://6017213752",
			Reset = "rbxassetid://10142422327",
		},
		Checkbox = {
			Active = "rbxassetid://6016251644",
			Inactive = "rbxassetid://6016251963",
		},
		Dropdown = {
			Arrow = "rbxassetid://10131770538",
		},
		Spinner = {
			Foreground = "rbxassetid://3222731032",
			Background = "rbxassetid://3222730627",
		},
		ScrollBar = {
			Top = "rbxassetid://6017290134",
			Middle = "rbxassetid://6017289904",
			Bottom = "rbxassetid://6017289712",
		},
		Circles = {
			[16] = "rbxassetid://3056541177",
			[32] = "rbxassetid://3088713341",
			[64] = "rbxassetid://4918677124",
			[128] = "rbxassetid://2600845734",
			[500] = "rbxassetid://2609138523"
		},
	},
	Sounds = {
		Notification = "rbxassetid://203785492",
	},
	StartSession = "",
	SessionActive = "",
	Configure = "",
}

local function guardForTypos(name, map)
	strict(name, map)

	for key, child in pairs(map) do
		if type(child) == "table" then
			guardForTypos(("%s.%s"):format(name, key), child)
		end
	end
end

guardForTypos("Assets", Assets)

return Assets
