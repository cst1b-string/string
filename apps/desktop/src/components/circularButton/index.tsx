import Link from "next/link";
import React from "react";

export const CircularChatButton: React.FC<{chatName: string}> = ({chatName}) => {
	// specific case for new chat button (because it has a different colour)
	if (chatName == "+"){
		return (
			<Link href="./newChat" className="text-center w-14 h-14 rounded-xl bg-lightGrey hover:bg-discordGreen text-discordGreen hover:text-white text-4xl">
				+
			</Link>
		)
	}
	else{
		// get first three words in chat name
		
		let chatNameInWords = chatName.split(" ", 2);
		let innerText = chatNameInWords.map(word => 
			word.charAt(0)).join("")
		return (
			<Link href={chatName} 
			// className="text-center w-14 h-14 rounded-xl bg-lightGrey hover:bg-hoverLightGrey text-white text-xl">
			className="flex items-center justify-center w-14 h-14 rounded-xl bg-lightGrey hover:bg-hoverLightGrey text-white text-xl">
				{innerText}
			</Link>
		)
	}
}