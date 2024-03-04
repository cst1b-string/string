import Link from "next/link";
import React from "react";

import { ChatTooltip } from "./tooltip";

export const CircularChatButton: React.FC<{ chatName: string }> = ({ chatName }) => {
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
		let innerText = chatNameInWords.map((word) => word.charAt(0)).join("");
		return (
			<div className="relative group">
				<button className="flex items-center justify-center w-14 h-14 min-h-14 rounded-xl bg-lightGrey hover:bg-hoverLightGrey text-white text-xl">
					{innerText}
				</button>
				{/* <ChatTooltip tooltip={chatName}/> */}
			</div>
		);
	}
};
