import React from "react";

export const ChatTooltip: React.FC<{tooltip: String}> = ({tooltip}) => {
	return (
		<div className="absolute bg-gray-700 text-white text-lg py-2 px-4 z-50 opacity-0 group-hover:opacity-100 transition-opacity">
			{tooltip}
		</div>
	)
}
