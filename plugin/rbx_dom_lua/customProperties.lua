local CollectionService = game:GetService("CollectionService")

--- A list of `Enum.Material` values that are used for Terrain.MaterialColors
local TERRAIN_MATERIAL_COLORS = {
	Enum.Material.Grass,
	Enum.Material.Slate,
	Enum.Material.Concrete,
	Enum.Material.Brick,
	Enum.Material.Sand,
	Enum.Material.WoodPlanks,
	Enum.Material.Rock,
	Enum.Material.Glacier,
	Enum.Material.Snow,
	Enum.Material.Sandstone,
	Enum.Material.Mud,
	Enum.Material.Basalt,
	Enum.Material.Ground,
	Enum.Material.CrackedLava,
	Enum.Material.Asphalt,
	Enum.Material.Cobblestone,
	Enum.Material.Ice,
	Enum.Material.LeafyGrass,
	Enum.Material.Salt,
	Enum.Material.Limestone,
	Enum.Material.Pavement,
}

-- Defines how to read and write properties that aren't directly scriptable.
--
-- The reflection database refers to these as having scriptability = "Custom"
return {
	Instance = {
		Attributes = {
			read = function(instance)
				return true, instance:GetAttributes()
			end,
			write = function(instance, _, value)
				local existing = instance:GetAttributes()

				for key, attr in pairs(value) do
					instance:SetAttribute(key, attr)
				end

				for key in pairs(existing) do
					if value[key] == nil then
						instance:SetAttribute(key, nil)
					end
				end

				return true
			end,
		},
		Tags = {
			read = function(instance)
				return true, CollectionService:GetTags(instance)
			end,
			write = function(instance, _, value)
				local existingTags = CollectionService:GetTags(instance)

				local unseenTags = {}
				for _, tag in ipairs(existingTags) do
					unseenTags[tag] = true
				end

				for _, tag in ipairs(value) do
					unseenTags[tag] = nil
					CollectionService:AddTag(instance, tag)
				end

				for tag in pairs(unseenTags) do
					CollectionService:RemoveTag(instance, tag)
				end

				return true
			end,
		},
	},
	LocalizationTable = {
		Contents = {
			read = function(instance, _)
				return true, instance:GetContents()
			end,
			write = function(instance, _, value)
				instance:SetContents(value)
				return true
			end,
		},
	},
	Model = {
		Scale = {
			read = function(instance, _, _)
				return true, instance:GetScale()
			end,
			write = function(instance, _, value)
				return true, instance:ScaleTo(value)
			end,
		},
	},
	Terrain = {
		MaterialColors = {
			read = function(instance: Terrain)
				-- There's no way to get a list of every color, so we have to
				-- make one.
				local colors = {}
				for _, material in TERRAIN_MATERIAL_COLORS do
					colors[material] = instance:GetMaterialColor(material)
				end

				return true, colors
			end,
			write = function(instance: Terrain, _, value: { [Enum.Material]: Color3 })
				for material, color in value do
					instance:SetMaterialColor(material, color)
				end
				return true
			end,
		},
	},
}
