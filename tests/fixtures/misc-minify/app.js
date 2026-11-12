/*! app.js v1.0 | (c) 2026 sample | MIT License */

// 장바구니 합계를 계산한다
function calculateTotal(shoppingCartItems) {
  var runningTotal = 0;
  for (var index = 0; index < shoppingCartItems.length; index++) {
    var currentItem = shoppingCartItems[index];
    runningTotal += currentItem.price * currentItem.quantity;
  }
  return runningTotal;
}

/* 결과를 화면에 쓴다 */
function render(totalElement, shoppingCartItems) {
  var formattedTotal = calculateTotal(shoppingCartItems).toFixed(2);
  totalElement.textContent = "합계: " + formattedTotal;
}
