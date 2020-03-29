local ReflectionDatabase = require(script.ReflectionDatabase)
local Error = require(script.Error)
local PropertyDescriptor = require(script.PropertyDescriptor)

local function findCanonicalPropertyDescriptor(className, propertyName)
	local currentClassName = className

	repeat
		local currentClass = ReflectionDatabase.classes[currentClassName]

		if currentClass == nil then
			return currentClass
		end

		local propertyData = currentClass.properties[propertyName]
		if propertyData ~= nil then
			if propertyData.isCanonical then
				return PropertyDescriptor.fromRaw(propertyData, currentClassName, propertyName)
			end

			if propertyData.canonicalName ~= nil then
				return PropertyDescriptor.fromRaw(
					currentClass.properties[propertyData.canonicalName],
					currentClassName,
					propertyData.canonicalName)
			end

			return nil
		end

		currentClassName = currentClass.superclass
	until currentClassName == nil

	return nil
end

local function readProperty(instance, propertyName)
	local descriptor = findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	if descriptor == nil then
		local fullName = ("%s.%s"):format(instance.className, propertyName)

		return false, Error.new(Error.Kind.UnknownProperty, fullName)
	end

	return descriptor:read(instance)
end

local function writeProperty(instance, propertyName, value)
	local descriptor = findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	if descriptor == nil then
		local fullName = ("%s.%s"):format(instance.className, propertyName)

		return false, Error.new(Error.Kind.UnknownProperty, fullName)
	end

	return descriptor:write(instance, value)
end

return {
	readProperty = readProperty,
	writeProperty = writeProperty,
	findCanonicalPropertyDescriptor = findCanonicalPropertyDescriptor,
	Error = Error,
	EncodedValue = require(script.EncodedValue),
}