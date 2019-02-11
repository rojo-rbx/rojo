local Assets = {
	Sprites = {
		WhiteCross = {
			asset = "rbxassetid://2738712459",
			offset = Vector2.new(190, 318),
			size = Vector2.new(18, 18),
		},
	},
	Slices = {
		RoundBox = {
			asset = "rbxassetid://2773204550",
			offset = Vector2.new(0, 0),
			size = Vector2.new(32, 32),
			center = Rect.new(4, 4, 4, 4),
		},
	},
	Images = {
		Logo = "rbxassetid://2773210620",
	},
	StartSession = "",
	SessionActive = "",
	Configure = "",
}

local function guardForTypos(name, map)
	setmetatable(map, {
		__index = function(_, key)
			error(("%q is not a valid member of %s"):format(tostring(key), name), 2)
		end
	})

	for key, child in pairs(map) do
		if type(child) == "table" then
			guardForTypos(("%s.%s"):format(name, key), child)
		end
	end
end

guardForTypos("Assets", Assets)

return Assets