local Error = require(script.Parent.Error)
local customProperties = require(script.Parent.customProperties)

-- A wrapper around a property descriptor from the reflection database with some
-- extra convenience methods.
--
-- The aim of this API is to facilitate looking up a property once, then reading
-- from it or writing to it multiple times. It's also useful when a consumer
-- wants to check additional constraints on the property before trying to use
-- it, like scriptability.
local PropertyDescriptor = {}
PropertyDescriptor.__index = PropertyDescriptor

local function get(container, key)
	return container[key]
end

local function set(container, key, value)
	container[key] = value
end

function PropertyDescriptor.fromRaw(data, className, propertyName)
	local key, value = next(data.DataType)

	return setmetatable({
		-- The meanings of the key and value in DataType differ when the type of
		-- the property is Enum. When the property is of type Enum, the key is
		-- the name of the type:
		--
		-- { Enum = "<name of enum>" }
		--
		-- When the property is not of type Enum, the value is the name of the
		-- type:
		--
		-- { Value = "<data type>" }
		dataType = key == "Enum" and key or value,

		scriptability = data.Scriptability,
		className = className,
		name = propertyName,
	}, PropertyDescriptor)
end

function PropertyDescriptor:read(instance)
	if self.scriptability == "ReadWrite" or self.scriptability == "Read" then
		local success, value = xpcall(get, debug.traceback, instance, self.name)

		if success then
			return success, value
		else
			return false, Error.new(Error.Kind.Roblox, value)
		end
	end

	if self.scriptability == "Custom" then
		if customProperties[self.className] == nil then
			local fullName = ("%s.%s"):format(instance.className, self.name)
			return false, Error.new(Error.Kind.PropertyNotReadable, fullName)
		end

		local interface = customProperties[self.className][self.name]

		return interface.read(instance, self.name)
	end

	if self.scriptability == "None" or self.scriptability == "Write" then
		local fullName = ("%s.%s"):format(instance.className, self.name)

		return false, Error.new(Error.Kind.PropertyNotReadable, fullName)
	end

	error(("Internal error: unexpected value of 'scriptability': %s"):format(tostring(self.scriptability)), 2)
end

function PropertyDescriptor:write(instance, value)
	if self.scriptability == "ReadWrite" or self.scriptability == "Write" then
		local success, err = xpcall(set, debug.traceback, instance, self.name, value)

		if success then
			return success
		else
			return false, Error.new(Error.Kind.Roblox, err)
		end
	end

	if self.scriptability == "Custom" then
		if customProperties[self.className] == nil then
			local fullName = ("%s.%s"):format(instance.className, self.name)
			return false, Error.new(Error.Kind.PropertyNotWritable, fullName)
		end

		local interface = customProperties[self.className][self.name]

		return interface.write(instance, self.name, value)
	end

	if self.scriptability == "None" or self.scriptability == "Read" then
		local fullName = ("%s.%s"):format(instance.className, self.name)

		return false, Error.new(Error.Kind.PropertyNotWritable, fullName)
	end
end

return PropertyDescriptor
