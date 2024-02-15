import Link from "next/link";

export const ChatButton = ( { chatName} : {chatName : string}) => {
	return (
		<Link href='/chat'>
			<button className="bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded">
				{chatName}
			</button>
		</Link>
	);
}
