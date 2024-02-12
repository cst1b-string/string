import Link from "next/link";

export const ExistingChat = ( { chatName} : {chatName : string}) => {
	return (
		<Link href='/chat'>
			<button className="bg-[#3363ff] text-[white] w-full py-4">
				{chatName}
			</button>
		</Link>
	);
}
