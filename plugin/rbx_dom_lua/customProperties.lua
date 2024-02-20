local CollectionService = game:GetService("CollectionService")
local ScriptEditorService = game:GetService("ScriptEditorService")

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
				local didAllWritesSucceed = true

				for attributeName, attributeValue in pairs(value) do
					local isNameValid =
						-- For our SetAttribute to succeed, the attribute name must be
						-- less than or equal to 100 characters...
						#attributeName <= 100
						-- ...must only contain alphanumeric characters, periods, hyphens,
						-- underscores, or forward slashes...
						and attributeName:match("[^%w%.%-_/]") == nil
						-- ... and must not use the RBX prefix, which is reserved by Roblox.
						and attributeName:sub(1, 3) ~= "RBX"

					if isNameValid then
						instance:SetAttribute(attributeName, attributeValue)
					else
						didAllWritesSucceed = false
					end
				end

				for key in pairs(existing) do
					if value[key] == nil then
						instance:SetAttribute(key, nil)
					end
				end

				return didAllWritesSucceed
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
		WorldPivotData = {
			read = function(instance)
				return true, instance:GetPivot()
			end,
			write = function(instance, _, value)
				if value == nil then
					return true, nil
				else
					return true, instance:PivotTo(value)
				end
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
	Script = {
		Source = {
			read = function(instance: Script)
				return true, ScriptEditorService:GetEditorSource(instance)
			end,
			write = function(instance: Script, _, value: string)
				task.spawn(function()
					ScriptEditorService:UpdateSourceAsync(instance, function()
						return value
					end)
				end)
				return true
			end,
		},
	},
	ModuleScript = {
		Source = {
			read = function(instance: ModuleScript)
				return true, ScriptEditorService:GetEditorSource(instance)
			end,
			write = function(instance: ModuleScript, _, value: string)
				task.spawn(function()
					ScriptEditorService:UpdateSourceAsync(instance, function()
						return value
					end)
				end)
				return true
			end,
		},
	},
}
