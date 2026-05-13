type DynamicDataPwProps = {
  "data-pw"?: string;
};

export function DynamicDataPw({ "data-pw": dataPw }: DynamicDataPwProps) {
  return <div data-pw={dataPw}>Dynamic</div>;
}

