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
		Logo = "rbxassetid://3405346157",
		Icon = "rbxassetid://3405341609",
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