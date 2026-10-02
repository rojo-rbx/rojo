--[[
	Counts how many of a virtual instance's properties match the live values on a
	candidate Roblox instance. `hydrate` uses this to break ties when several
	existing children share the same Name and ClassName.

	This mirrors the read -> decode -> compare flow that `diff` uses, reusing the
	same `getProperty`, `decodeValue`, and `trueEquals` helpers.
]]

local getProperty = require(script.Parent.getProperty)
local decodeValue = require(script.Parent.decodeValue)
local trueEquals = require(script.Parent.trueEquals)

local function countMatchingProperties(instance, virtualInstance, instanceMap)
	local score = 0

	for propertyName, virtualValue in virtualInstance.Properties do
		-- Skip refs. During hydration the instanceMap is still being built
		-- top-down, so a ref may point at an instance we haven't hydrated yet
		-- and therefore can't decode reliably. Refs are also a poor
		-- disambiguator between same-named siblings.
		if next(virtualValue) == "Ref" then
			continue
		end

		local getSuccess, existingValue = getProperty(instance, propertyName)
		if not getSuccess then
			continue
		end

		local decodeSuccess, decodedValue = decodeValue(virtualValue, instanceMap)
		if not decodeSuccess then
			continue
		end

		if trueEquals(existingValue, decodedValue) then
			score += 1
		end
	end

	return score
end

return countMatchingProperties
