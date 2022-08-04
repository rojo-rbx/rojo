--[[
	Attempts to read a property from the given instance.
]]

local Packages = script.Parent.Parent.Parent.Packages
local RbxDom = require(Packages.RbxDom)
local Error = require(script.Parent.Error)

local function getProperty(instance, propertyName)
	local descriptor = RbxDom.findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	-- We can skip unknown properties; they're not likely reflected to Lua.
	--
	-- A good example of a property like this is `Model.ModelInPrimary`, which
	-- is serialized but not reflected to Lua.
	if descriptor == nil then
		return false, Error.new(Error.UnknownProperty, {
			className = instance.ClassName,
			propertyName = propertyName,
		})
	end

	if descriptor.scriptability == "None" or descriptor.scriptability == "Write" then
		return false, Error.new(Error.UnreadableProperty, {
			className = instance.ClassName,
			propertyName = propertyName,
		})
	end

	local success, valueOrErr = descriptor:read(instance)

	if not success then
		local err = valueOrErr

		-- If we don't have permission to read a property, we can chalk that up
		-- to our database being out of date and the engine being right.
		if err.kind == RbxDom.Error.Kind.Roblox and err.extra:find("lacking permission") then
			return false, Error.new(Error.LackingPropertyPermissions, {
				className = instance.ClassName,
				propertyName = propertyName,
			})
		end

		if err.kind == RbxDom.Error.Kind.Roblox and err.extra:find("is not a valid member of") then
			return false, Error.new(Error.UnknownProperty, {
				className = instance.ClassName,
				propertyName = propertyName,
			})
		end

		return false, Error.new(Error.OtherPropertyError, {
			className = instance.ClassName,
			propertyName = propertyName,
		})
	end

	return true, valueOrErr
end

return getProperty
