local Config = require(script.Parent.Config)

local function getValueContainer()
	return game:FindFirstChild("RojoDev-" .. Config.codename)
end

local valueContainer = getValueContainer()

local function getValue(name)
	if valueContainer == nil then
		return nil
	end

	local valueObject = valueContainer:FindFirstChild(name)

	if valueObject == nil then
		return nil
	end

	return valueObject.Value
end

local function setValue(name, kind, value)
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
		valueContainer.Name = "RojoDev-" .. Config.codename
		valueContainer.Parent = game
	end

	setValue("LogLevel", "IntValue", getValue("LogLevel") or 2)
end

_G[("ROJO_%s_DEV_CREATE"):format(Config.codename:upper())] = createAllValues

local DevSettings = {}

function DevSettings:isEnabled()
	return valueContainer ~= nil
end

function DevSettings:getLogLevel()
	return getValue("LogLevel")
end

return DevSettings