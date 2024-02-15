import ChatSidebar from "@/components/chatSidebar";

export default function Home () {
  return (
	<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
		<div className="">
			<ChatSidebar />
		</div>

		<div className="col-span-2 text-white font-bold "> 
			Chat window will go here
		</div>
	</div>
  );
}
