local InstanceMap = require(script.Parent.InstanceMap)

local Module = {}

local function randomName(len)
	local chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
	local out = {}

	for i = 1, len do
		local idx = math.random(1, #chars)
		out[i] = string.sub(chars, idx, idx)
	end

	return table.concat(out)
end

-- Очень примитивная функция для поиска локальных переменных
local function findLocals(Source: string)
	local found = {}

	-- локальные переменные
	for var in Source:gmatch("local%s+([%a_][%w_]*)") do
		if not found[var] and var ~= "function" then
			found[var] = randomName(8)
		end
	end

	-- локальные функции
	for func in Source:gmatch("local%s+function%s+([%a_][%w_]*)") do
		if not found[func] then
			found[func] = randomName(8)
		end
	end

	return found
end

function Module:Obfuscate(Source: string)
	-- Убираем комментарии
	Source = Source:gsub("%-%-%[%[.-%]%]", "") -- многострочные
	Source = Source:gsub("%-%-.-\n", "\n") -- однострочные

	-- Находим локальные переменные
	local vars = findLocals(Source)

	-- Заменяем их на рандомные
	for orig, obf in pairs(vars) do
		-- чтобы избежать пересечения слов,
		-- заменяем как отдельные токены
		Source = Source:gsub("(%f[%w_])" .. orig .. "(%f[^%w_])", "%1" .. obf .. "%2")
	end

	-- Убираем лишние пробелы
	Source = Source:gsub("%s+", " ")

	-- Убираем пробелы перед \n
	Source = Source:gsub(" %\n", "\n")

	-- Сжимаем переносы
	Source = Source:gsub("\n+", "\n")

	return Source
end

-- EXAMPLE
-- for _, update in ipairs(patch.updated) do
--     local instance = self.__instanceMap.fromIds[update.id]

--     if instance and (instance:IsA("Script") or instance:IsA("LocalScript") or instance:IsA("ModuleScript")) then
--         -- Подключаем модуль обфускатора
--         local Obfuscator = require(script.Parent.Obfuscator)
--         instance.Source = Obfuscator.Obfuscate(instance.Source)
--     end
-- end

function Module:ObfuscatePatch(ServeSession, Patch)
	if Patch.added ~= nil then
		for _, add in pairs(Patch.added) do
			if add.Properties.Source ~= nil then
				task.spawn(function()
					local instance = ServeSession.__instanceMap.fromIds[add.Parent]:FindFirstChild(add.Name)

					if instance ~= nil then
						instance.Source = self:Obfuscate(add.Properties.Source.String)
					end

					-- add.Properties.Source.String = self:Obfuscate(add.Properties.Source.String)
				end)
			end
		end
	end

	if Patch.updated ~= nil then
		for _, update in ipairs(Patch.updated) do
			if update.changedProperties.Source ~= nil then
				task.spawn(function()
					local instance = ServeSession.__instanceMap.fromIds[update.id]

					instance.Source = self:Obfuscate(update.changedProperties.Source.String)

					-- update.changedProperties.Source.String = self:Obfuscate(update.changedProperties.Source.String)
				end)
			end
		end
	end
end

return Module
