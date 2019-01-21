local Config = require(script.Parent.Config)

local VALUES = {
	LogLevel = {
		type = "IntValue",
		defaultUserValue = 2,
		defaultDevValue = 3,
	},
}

local CONTAINER_NAME = "RojoDevSettings" .. Config.codename

local function getValueContainer()
	return game:FindFirstChild(CONTAINER_NAME)
end

local valueContainer = getValueContainer()

local function getStoredValue(name)
	if valueContainer == nil then
		return nil
	end

	local valueObject = valueContainer:FindFirstChild(name)

	if valueObject == nil then
		return nil
	end

	return valueObject.Value
end

local function setStoredValue(name, kind, value)
	local object = valueContainer:FindFirstChild(name)

	if object == nil then
		object = Instance.new(kind)
		object.Name = name
		object.Parent = valueContainer
	end

	object.Value = value
end

local function createAllValues()
	valueContainer = getValueContainer()

	if valueContainer == nil then
		valueContainer = Instance.new("Folder")
		valueContainer.Name = CONTAINER_NAME
		valueContainer.Parent = game
	end

	for name, value in pairs(VALUES) do
		setStoredValue(name, value.type, value.defaultDevValue)
	end
end

_G[("ROJO_%s_DEV_CREATE"):format(Config.codename:upper())] = createAllValues

local DevSettings = {}

function DevSettings:isEnabled()
	return valueContainer ~= nil
end

function DevSettings:getLogLevel()
	return getStoredValue("LogLevel") or VALUES.LogLevel.defaultUserValue
end

return DevSettings