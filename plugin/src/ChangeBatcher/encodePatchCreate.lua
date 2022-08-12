local HttpService = game:GetService("HttpService")


local Packages = script.Parent.Parent.Parent.Packages
local RbxDom = require(Packages.RbxDom)

local  function createInstanceSnapshot(instanceMap,instance) 

	local id = HttpService:GenerateGUID(false)

	instanceMap.createdInstances[id] = instance

	local children = {}
	for i,child in ipairs(instance:GetChildren()) do
		table.insert(children,createInstanceSnapshot(instanceMap,child))
	end

	local success, properties = RbxDom.findAllNoneDefaultPropertiesEncoded(instance)
	if success
		properties.Name = nil
		properties.ClassName = nil

		local attributes = nil
		if #properties.Attributes ~= 0 then
			attributes = properties.Attributes
		end

		properties.Attributes = nil
		if #properties == 0 then
			properties = nil
		end
		
		return {
			Name = instance.Name,
			ClassName = instance.ClassName,
			Children = children,
			Properties = properties,
			attributes = attributes,
			DebugId = id
		}
	end
end

return function(instanceMap,instance,parentId)
	return {
		id = parentId,
		instance = createInstanceSnapshot(instanceMap,instance),
	}
end
