local Log = require(script.Parent.Parent.Parent.Log)
local RbxDom = require(script.Parent.Parent.Parent.RbxDom)

return function(instance, propertyName, propertyDescriptor)
	local readSuccess, readResult = propertyDescriptor:read(instance)

	if not readSuccess then
		Log.warn("Could not sync back property {:?}.{}: {}", instance, propertyName, readResult)
		return false, nil
	end

	local dataType = propertyDescriptor.dataType
	local encodeSuccess, encodeResult = RbxDom.EncodedValue.encode(readResult, dataType)

	if not encodeSuccess then
		Log.warn("Could not sync back property {:?}.{}: {}", instance, propertyName, encodeResult)
		return false, nil
	end

	return true, encodeResult
end
