import Link from "next/link";
import React from "react";

export const CircularChatButton: React.FC<{ chatName: string; isExpanded: boolean }> = ({
	chatName,
	isExpanded,
}) => {
	// specific case for new chat button (because it has a different colour)
	if (chatName == "+") {
		return (
			<Link href="./newChat">
				<div className={`flex  items-center ${isExpanded ? "min-w-50" : "min-w-14"}`}>
					<button
						className={`flex items-center justify-center min-h-14 w-14  h-14 rounded-xl bg-darkSidebar  text-white  text-4xl`}
					>
						+ 
					</button>
					{isExpanded && (
						<span className="text-white font-semibold ml-3 mr-2 overflow-visible whitespace-pre">
							New Chat
						</span>
					)}
				</div>
			</Link>
			
		);
	} else {
		// get first three words in chat name
		let chatNameInWords = chatName.split(" ", 3);
		// then get the first letter from these words
		let innerText = chatNameInWords.map((word) => word.charAt(0)).join("");

		let serverName = chatName.length >= 10 ? chatName.slice(0, 8) + "..." : chatName.slice(0, 8);
		return (
			<div className={`flex items-center ${isExpanded ? "min-w-50" : "min-w-14"}`}>
				<button className="flex items-center justify-center min-w-14 h-14 rounded-xl bg-darkCircularChatButton text-white text-xl">
					{innerText}
				</button>
				{isExpanded && (
					<span className="text-white font-semibold ml-3 mr-2 overflow-visible whitespace-pre">
						{serverName}
					</span>
				)}
			</div>
		);
	}
};
