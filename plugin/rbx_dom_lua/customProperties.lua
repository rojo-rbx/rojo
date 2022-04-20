local CollectionService = game:GetService("CollectionService")
local setAttribute = require(script.Parent.setAttribute)

-- Defines how to read and write properties that aren't directly scriptable.
-- The reflection database refers to these as having scriptability = "Custom"

return {
	Instance = {
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

		Attributes = {
			read = function(instance)
				return true, instance:GetAttributes()
			end,

			write = function (instance, _, attributes)
				local existingAttributes = instance:GetAttributes()
				local unseenAttributes = {}

				for name in pairs(existingAttributes) do
					unseenAttributes[name] = true
				end

				for name, value in pairs(attributes) do
					local ok, err = setAttribute(instance, name, value)

					if ok then
						unseenAttributes[name] = nil
					else
						return false, err
					end
				end

				for name in pairs(unseenAttributes) do
					instance:SetAttribute(name, nil)
				end

				return true
			end,
		}
	},
	
	LocalizationTable = {
		Contents = {
			read = function(instance)
				return true, instance:GetContents()
			end,

			write = function(instance, _, value)
				instance:SetContents(value)
				return true
			end,
		},
	},
}
