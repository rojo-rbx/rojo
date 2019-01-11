local sheetAsset = "rbxassetid://2738712459"

local Assets = {
	Sprites = {
		WhiteCross = {
			asset = sheetAsset,
			offset = Vector2.new(190, 318),
			size = Vector2.new(18, 18),
		},
	},
	Slices = {
		GrayBox = {
			asset = sheetAsset,
			offset = Vector2.new(147, 433),
			size = Vector2.new(38, 36),
			center = Rect.new(8, 8, 9, 9),
		},
		GrayButton02 = {
			asset = sheetAsset,
			offset = Vector2.new(0, 98),
			size = Vector2.new(190, 45),
			center = Rect.new(16, 16, 17, 17),
		},
		GrayButton07 = {
			asset = sheetAsset,
			offset = Vector2.new(195, 0),
			size = Vector2.new(49, 49),
			center = Rect.new(16, 16, 17, 17),
		},
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