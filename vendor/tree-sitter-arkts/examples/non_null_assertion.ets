/**
 * 非空断言运算符测试文件
 * Non-null Assertion Operator Test
 * 
 * 语法：expression!
 * 作用：告诉TypeScript/ArkTS编译器某个值不为null或undefined
 */

// 基本用法
let value1 = obj.property!;
let value2 = arr[0]!;
let value3 = func()!;

// 链式调用
let result = obj.method()!.property;
let chain = obj!.property!.method()!;

// 与其他运算符组合
let withAs = (value as string)!;
let withMember = response.data!.id;

// 实际场景：HuaweiID授权
let authorizationWithHuaweiIDCredential = authorizationWithHuaweiIDResponse.data!;

// 可选链与非空断言
let optional = obj?.property!;
let optionalChain = obj?.method()!.value;
