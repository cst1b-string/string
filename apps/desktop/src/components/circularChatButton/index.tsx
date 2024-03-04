import Link from "next/link";
import React from "react";

import { ChatTooltip } from "./tooltip";

export const CircularChatButton: React.FC<{ chatName: string; isExpanded: boolean }> = ({
	chatName,
	isExpanded,
}) => {
	// specific case for new chat button (because it has a different colour)
	if (chatName == "+") {
		return (
			<div>
				<Link
					href="./newChat"
					className="flex items-center justify-center min-h-14 w-14 h-14 rounded-xl bg-lightGrey hover:bg-discordGreen text-discordGreen hover:text-white text-4xl"
				>
					+
				</Link>
				{/* <ChatTooltip tooltip={chatName}/> */}
			</div>
		);
	} else {
		// get first three words in chat name
		let chatNameInWords = chatName.split(" ", 3);
		// then get the first letter from these words
		let innerText = chatNameInWords.map((word) => word.charAt(0)).join("");

		let serverName = chatName.length >= 10 ? chatName.slice(0, 8) + "...": chatName.slice(0, 8);
		return (
			<div className={`flex items-center ${isExpanded ? "min-w-50": "min-w-14"}`}>
				<Link
					href={chatName}
					className="flex items-center justify-center min-w-14 h-14 min-h-14 rounded-xl bg-lightGrey hover:bg-hoverLightGrey text-white text-xl"
				>
					{innerText}
				</Link>
				{isExpanded && (
					<span className="text-white font-semibold ml-3 mr-2 overflow-visible whitespace-pre">{serverName}</span>
				)}
				{/* <ChatTooltip tooltip={chatName}/> */}
			</div>
		);
	}
};
