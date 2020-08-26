local CollectionService = game:GetService("CollectionService")
local HttpService = game:GetService("HttpService")

local function getContents(localizationTable)
	local array = {}
	for index, entry in ipairs(localizationTable:GetEntries()) do
		local newEntry = {}
		for key, value in next, entry do
			if key == "Context" or key == "Example" then
				continue
			end

			newEntry[string.lower(key)] = value
		end

		array[index] = newEntry
	end

	return HttpService:JSONEncode(array)
end

local function setContents(localizationTable, contents)
	local newContents = {}
	for index, entry in ipairs(HttpService:JSONDecode(contents)) do
		local newEntry = {
			Context = "";
			Example = "";
		}

		for key, value in next, entry do
			newEntry[string.gsub(key, "^%a", string.upper)] = value
		end

		newContents[index] = newEntry
	end

	localizationTable:SetEntries(newContents)
end

-- Defines how to read and write properties that aren't directly scriptable.
--
-- The reflection database refers to these as having scriptability = "Custom"
return {
	Instance = {
		Tags = {
			read = function(instance)
				local tagList = CollectionService:GetTags(instance)

				return true, table.concat(tagList, "\0")
			end,
			write = function(instance, _, value)
				local existingTags = CollectionService:GetTags(instance)

				local unseenTags = {}
				for _, tag in ipairs(existingTags) do
					unseenTags[tag] = true
				end

				local tagList = string.split(value, "\0")
				for _, tag in ipairs(tagList) do
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
			read = function(instance)
				return true, getContents(instance)
			end,
			write = function(instance, _, value)
				setContents(instance, value)
				return true
			end,
		},
	},
}
